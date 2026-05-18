#![cfg_attr(docsrs, doc(cfg(feature = "search")))]
//! Text searching functionality
//!
//! Text searching inside minus is quite advanced than other terminal pagers. It is highly
//! inspired by modern text editors and hence provides features like:-
//! - [Keybindings](../index.html#key-bindings-available-at-search-prompt) similar to modern text editors
//! - Incremental search
//! - Full regex support for writing advanced search queries
//!   and more...
//!
//! # Incremental Search
//! minus supports incrementally searching the text. This means that you can view the search
//! matches inside the text match as soon as you start typing the query.
//!
//! It is also significant because minus caches a lot of results from each incremental search run
//! and then reuses those results when the search query is confirmed by pressing `Enter`. This
//! approach eliminates the need to re run the search of text after confirming the query.
//!
//! Running Incremental search can be controlled by a function. The function should take reference
//! to [`SearchOpts`] and `&str` containing the currently entered query as arguments and return a bool
//! as output. This way we can impose a condition so that incremental search does not get really
//! resource intensive for really vague queries This also allows applications can control whether
//! they want incremental search to run. By default minus uses a default condition where incremental
//! search runs only when length of search query is greater than 1 and number of screen lines (lines
//! obtained after taking care of wrapping, mapped to a single row on the terminal) is greater than
//! 5000.
//!
//! Applications can override this condition with the help of
//! [`Pager::set_incremental_search_condition`](crate::pager::Pager::set_incremental_search_condition) function.
//!
//! Here is a an example to demonstrate on its usage. Here we set the condition to run incremental
//! search only when the length of the search query is greater than 1.
//! ```
//! use minus::{Pager, search::SearchOpts};
//!
//! let pager = Pager::new();
//! pager.set_incremental_search_condition(Box::new(|_, line: &str| line.len() > 1)).unwrap();
//! ```
//! To completely disable incremental search, set the condition to false
//! ```
//! use minus::{Pager, search::SearchOpts};
//!
//! let pager = Pager::new();
//! pager.set_incremental_search_condition(Box::new(|_, _| false)).unwrap();
//! ```
//! Similarly to always run incremental search, set the condition to true
//! ```
//! use minus::{Pager, search::SearchOpts};
//!
//! let pager = Pager::new();
//! pager.set_incremental_search_condition(Box::new(|_, _| true)).unwrap();
//! ```

use crate::minus_core::utils::{LinesRowMap, display, term};
use crate::screen::Screen;
use crate::{LineNumbers, PagerState};
use crate::{error::MinusError, minus_core::utils, screen};
use crossterm::{
    cursor,
    style::Attribute,
    terminal::{Clear, ClearType},
};
use regex::Regex;
use rustyline::completion::Completer;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context, Editor, Helper, error::ReadlineError};
use std::collections::BTreeSet;
use std::{
    borrow::Cow,
    convert::TryInto,
    fmt,
    io::Write,
    sync::{LazyLock, Mutex},
};

static INVERT: LazyLock<String> = LazyLock::new(|| Attribute::Reverse.to_string());
static NORMAL: LazyLock<String> = LazyLock::new(|| Attribute::NoReverse.to_string());
static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("[\\u001b\\u009b]\\[[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]")
        .unwrap()
});

#[derive(Clone, Copy, Debug, Default, Eq)]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
#[allow(clippy::module_name_repetitions)]
/// Defines modes in which the search can run
pub enum SearchMode {
    /// Find matches from or after the current page
    Forward,
    /// Find matches before the current page
    Reverse,
    /// No search active
    #[default]
    Unknown,
}

impl PartialEq for SearchMode {
    fn eq(&self, other: &Self) -> bool {
        core::mem::discriminant(self) == core::mem::discriminant(other)
    }
}

/// Options controlling the behaviour of search overall
///
/// Although it isn't much important for most use cases but it alongside [`IncrementalSearchOpts`] are the key components
/// when applications want to customize the incremental seaech condition.
///
/// Most of the fields have self-explanatory names so it should be very easy to get started using
/// this
#[allow(clippy::module_name_repetitions)]
pub struct SearchOpts<'a> {
    /// Direction of search. See [`SearchMode`].
    pub search_mode: SearchMode,
    /// Number of rows available in the terminal
    pub rows: u16,
    /// Number of cols available in the terminal
    pub cols: u16,
    /// Options specifically controlling incremental search
    pub incremental_search_options: Option<IncrementalSearchOpts<'a>>,
    compiled_regex: Option<Regex>,
}

/// Options to control incremental search
pub struct IncrementalSearchOpts<'a> {
    /// Current status of line numbering
    pub line_numbers: LineNumbers,
    /// Value of [`PagerState::upper_mark`] before starting of search prompt
    pub initial_upper_mark: usize,
    /// Reference to [`PagerState::screen`]
    pub screen: &'a Screen,
    /// Cached map from logical lines to formatted rows.
    pub lines_to_row_map: &'a LinesRowMap,
    /// Value of [`PagerState::upper_mark`] before starting of search prompt
    pub initial_left_mark: usize,
    /// Value of [`PagerState::cols`]
    cols: usize,
    /// Value of [`PagerState::rows`] - 1 and 0 if rows is 0.
    writable_rows: usize,
}

impl<'a> From<&'a PagerState> for IncrementalSearchOpts<'a> {
    fn from(ps: &'a PagerState) -> Self {
        Self {
            line_numbers: ps.line_numbers,
            initial_upper_mark: ps.upper_mark,
            screen: &ps.screen,
            lines_to_row_map: &ps.lines_to_row_map,
            initial_left_mark: ps.left_mark,
            cols: ps.cols,
            writable_rows: ps.rows.saturating_sub(1),
        }
    }
}

impl IncrementalSearchOpts<'_> {
    const fn line_number_digits(&self) -> usize {
        utils::digits(self.screen.line_count())
    }
}

#[allow(clippy::fallible_impl_from)]
impl<'a> From<&'a PagerState> for SearchOpts<'a> {
    fn from(ps: &'a PagerState) -> Self {
        let incremental_search_options = IncrementalSearchOpts::from(ps);

        Self {
            rows: ps.rows.try_into().unwrap(),
            cols: ps.cols.try_into().unwrap(),
            incremental_search_options: Some(incremental_search_options),
            compiled_regex: None,
            search_mode: ps.search_state.search_mode,
        }
    }
}

/// Return type of [`fetch_input`]
pub(crate) struct FetchInputResult {
    /// Original search query
    pub(crate) string: String,
    /// Cached pre-compiled [`Regex`] if available
    pub(crate) compiled_regex: Option<Regex>,
}

impl FetchInputResult {
    /// Create an empty `FetchInputResult` with string set to empty string and
    /// `incremental_search_cache` and `compiled_regex` set to `None`.
    const fn new_empty() -> Self {
        Self {
            string: String::new(),
            compiled_regex: None,
        }
    }
}

fn line_matches_query(line: &str, query: &Regex) -> bool {
    let stripped = ANSI_REGEX.replace_all(line, "");
    query.is_match(stripped.as_ref())
}

fn preview_line<'a>(
    iso: &IncrementalSearchOpts<'_>,
    query: &Regex,
    line_idx: usize,
    line: &'a str,
    visible_lines: &mut Vec<Cow<'a, str>>,
    upper_mark: &mut Option<usize>,
    wrapped: bool,
) {
    // Skip all lines that don't have any match
    if upper_mark.is_none() && !line_matches_query(line, query) {
        return;
    }

    let row_start = *iso.lines_to_row_map.get(line_idx).unwrap_or(&0);
    let mut match_row_idx = None;
    let mut formatted_rows = screen::format_line(
        line,
        iso.line_number_digits(),
        line_idx,
        iso.line_numbers,
        iso.cols,
        iso.screen.line_wrapping,
    )
    .enumerate()
    .map(|(i, fr)| {
        let h = highlight_matches_args(&fr.row, query, false);
        if h.is_match {
            if wrapped || line_idx + i > iso.initial_upper_mark {
                match_row_idx = Some(line_idx + i);
            }
            Cow::Owned(format!("{h}"))
        } else {
            fr.row
        }
    })
    .collect::<Vec<Cow<str>>>();

    if upper_mark.is_none() {
        if match_row_idx.is_none() {
            return;
        }
        let match_row_idx = match_row_idx.unwrap();
        let skip_rows = match_row_idx.saturating_sub(row_start);
        *upper_mark = Some(match_row_idx);
        visible_lines.extend(formatted_rows.drain(skip_rows..));
    } else {
        visible_lines.append(&mut formatted_rows);
    }

    if visible_lines.len() >= iso.writable_rows {
        visible_lines.truncate(iso.writable_rows);
    }
}

fn incremental_preview<'a>(
    iso: &IncrementalSearchOpts<'a>,
    query: &'a Regex,
) -> Option<Vec<Cow<'a, str>>> {
    if iso.writable_rows == 0 {
        return None;
    }

    let start_line_idx = iso.lines_to_row_map.row_to_line(iso.initial_upper_mark)?;
    let mut visible_lines: Vec<Cow<str>> = Vec::with_capacity(iso.writable_rows);
    let mut upper_mark = None;

    for (line_idx, line) in iso
        .screen
        .orig_text
        .lines()
        .enumerate()
        .skip(start_line_idx)
    {
        preview_line(
            iso,
            query,
            line_idx,
            line,
            &mut visible_lines,
            &mut upper_mark,
            false,
        );
        if visible_lines.len() >= iso.writable_rows {
            break;
        }
    }

    // visible_lines places the first search march as its first element. However if the match is
    // near the EOF, it might not fill up completely and show blank lines on the display.
    // We fix this by filling visible_lines by as many rows such that a pageful of data can be
    // displayed.
    if let Some(um) = upper_mark
        && visible_lines.len() < iso.writable_rows
        && iso.screen.formatted_lines_count() > iso.writable_rows
    {
        let start = iso
            .screen
            .formatted_lines_count()
            .saturating_sub(iso.writable_rows);
        let to_insert = um.saturating_sub(start);
        let shift = visible_lines.len();
        for l in iso
            .screen
            .formatted_lines
            .iter()
            .skip(start)
            .take(to_insert)
        {
            visible_lines.push(Cow::Borrowed(l.as_str()));
        }
        visible_lines.rotate_left(shift);
    }

    if upper_mark.is_none() {
        for (line_idx, line) in iso
            .screen
            .orig_text
            .lines()
            .enumerate()
            .take(start_line_idx)
        {
            preview_line(
                iso,
                query,
                line_idx,
                line,
                &mut visible_lines,
                &mut upper_mark,
                true,
            );
            if visible_lines.len() >= iso.writable_rows {
                break;
            }
        }
    }

    if upper_mark.is_some() {
        Some(visible_lines)
    } else {
        None
    }
}

/// Runs the incremental search
///
/// It will return if `Ok(SomeIncrementalSearchCache)` if there was a successful run of incremental
/// search otherwise it will return `Ok(None)`.
///
/// # Errors
/// This function will returns a `Err(MinusError)` if any operation on the terminal failed to
/// execute.
fn run_incremental_search<'a, F, O>(
    out: &mut O,
    so: &'a SearchOpts<'a>,
    line: &'a str,
    incremental_search_condition: F,
) -> crate::Result<()>
where
    O: Write,
    F: Fn(&'a SearchOpts, &'a str) -> bool,
{
    let Some(iso) = so.incremental_search_options.as_ref() else {
        return Ok(());
    };
    let screen = iso.screen;
    let line_numbers = iso.line_numbers;
    let initial_upper_mark = iso.initial_upper_mark;
    let initial_left_mark = iso.initial_left_mark;

    // Check if we can continue forward with incremental search
    let should_proceed = so.compiled_regex.is_some() && incremental_search_condition(so, line);

    // **Screen resetting**:
    // This is an important bit when running incremental search.It reset the terminal screen to
    // display the lines from the same location and in the same way as before the search even
    // started. Basically print it exactly how it looked before pressing `/` or `?`,
    let reset_screen = |out: &mut O, so: &SearchOpts<'_>| -> crate::Result {
        display::write_text_checked(
            out,
            &screen.formatted_lines,
            initial_upper_mark,
            so.rows.into(),
            so.cols.into(),
            screen.line_wrapping,
            initial_left_mark,
            line_numbers,
            screen.line_count(),
        )?;
        Ok(())
    };

    // If the query prior to the current one had a successful incremental search run and now the
    // current query isn't a valid regex or the incremental search condition has returned false
    // then
    if !should_proceed {
        reset_screen(out, so)?;
        return Ok(());
    }

    let query = so.compiled_regex.as_ref().unwrap();

    let Some(visible_lines) = incremental_preview(iso, query) else {
        reset_screen(out, so)?;
        return Ok(());
    };

    // Draw the incrementally searched lines from upper mark
    display::write_text_checked(
        out,
        &visible_lines,
        0,
        so.rows.into(),
        so.cols.into(),
        iso.screen.line_wrapping,
        iso.initial_left_mark,
        iso.line_numbers,
        iso.screen.line_count(),
    )?;

    Ok(())
}

// HACK: GET the bare `Write` trait to be `Send` + `Sync` without leaving the lock
struct ThreadSafeWriter<'a>(*mut (dyn Write + 'a));

unsafe impl Send for ThreadSafeWriter<'_> {}
unsafe impl Sync for ThreadSafeWriter<'_> {}

impl Write for ThreadSafeWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        unsafe { (*self.0).write(buf) }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        unsafe { (*self.0).flush() }
    }
}

struct SearchHelper<'a> {
    out: Mutex<ThreadSafeWriter<'a>>,
    search_opts: Mutex<SearchOpts<'a>>,
    incremental_search_condition: &'a (dyn Fn(&SearchOpts, &str) -> bool + Send + Sync),
}

impl Helper for SearchHelper<'_> {}

impl Highlighter for SearchHelper<'_> {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let mut out = self.out.lock().unwrap();
        let mut so = self.search_opts.lock().unwrap();

        so.compiled_regex = Regex::new(line).ok();

        let _ = run_incremental_search(&mut *out, &so, line, self.incremental_search_condition);

        let _ = term::move_cursor(&mut *out, 0, so.rows, false);
        let _ = out.flush();
        drop(out);
        drop(so);

        Cow::Borrowed(line)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: CmdKind) -> bool {
        true
    }
}

impl Validator for SearchHelper<'_> {
    fn validate(&self, _ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
    fn validate_while_typing(&self) -> bool {
        false
    }
}

impl Hinter for SearchHelper<'_> {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Completer for SearchHelper<'_> {
    type Candidate = String;
}

/// Fetch the search query
///
/// Uses rustyline for prompt input.
#[cfg(feature = "search")]
pub(crate) fn fetch_input(
    out: &mut impl std::io::Write,
    ps: &PagerState,
) -> Result<FetchInputResult, MinusError> {
    // Set the search character to show at column 0
    let search_char = if ps.search_state.search_mode == SearchMode::Forward {
        "/"
    } else {
        "?"
    };

    // Initial setup
    // - Place the cursor at the beginning of prompt line
    // - Clear the prompt
    // - Show the cursor
    term::move_cursor(out, 0, ps.rows.try_into().unwrap(), false)?;
    write!(out, "{}{}", Clear(ClearType::CurrentLine), cursor::Show)?;
    crossterm::execute!(out, crossterm::event::DisableMouseCapture)?;
    out.flush()?;

    let mut readline = Editor::<SearchHelper<'_>, _>::new().unwrap();
    let search_opts = SearchOpts::from(ps);
    let writer_ptr: *mut dyn std::io::Write = std::ptr::from_mut(out);
    readline.set_helper(Some(SearchHelper {
        out: Mutex::new(ThreadSafeWriter(writer_ptr)),
        search_opts: Mutex::new(search_opts),
        incremental_search_condition: &*ps.search_state.incremental_search_condition,
    }));

    let prompt = readline.readline(search_char);

    // Teardown: almost opposite of setup
    let helper = readline.helper_mut().unwrap();
    let mut out_lock = helper.out.lock().unwrap();
    term::move_cursor(&mut *out_lock, 0, ps.rows.try_into().unwrap(), false)?;
    write!(
        &mut *out_lock,
        "{}{}",
        Clear(ClearType::CurrentLine),
        cursor::Hide
    )?;
    crossterm::execute!(&mut *out_lock, crossterm::event::EnableMouseCapture)?;
    out_lock.flush()?;
    drop(out_lock);

    match prompt {
        Ok(str) => {
            let mut so = helper.search_opts.lock().unwrap();
            Ok(FetchInputResult {
                compiled_regex: so.compiled_regex.take(),
                string: str,
            })
        }
        Err(ReadlineError::Io(e)) => Err(MinusError::from(e)),
        Err(ReadlineError::Errno(_) | ReadlineError::Signal(_)) => todo!(),
        Err(ReadlineError::Interrupted | ReadlineError::Eof | _) => {
            Ok(FetchInputResult::new_empty())
        }
    }
}

pub(crate) fn highlight_matches_args<'a, 'b>(
    line: &'a str,
    query: &'b Regex,
    accurate: bool,
) -> HighlightMatchesArgs<'a, 'b> {
    let stripped_str = ANSI_REGEX.replace_all(line, "");
    let is_match = query.is_match(&stripped_str);
    HighlightMatchesArgs {
        line,
        query,
        accurate,
        is_match,
    }
}

fn highlight_line_matches_ansi(line: &str, query: &regex::Regex, accurate: bool) -> String {
    let stripped_str = ANSI_REGEX.replace_all(line, "");

    // if it doesn't match, don't even try. Just return.
    if !query.is_match(&stripped_str) {
        return line.to_string();
    }

    // sum_width is used to calculate the total width of the ansi escapes
    // up to the point in the original string where it is being used
    let mut sum_width = 0;

    // find all ansi escapes in the original string, and map them
    // to a Vec<(usize, &str)> where
    //   .0 == the start index in the STRIPPED string
    //   .1 == the escape sequence itself
    let escapes = ANSI_REGEX
        .find_iter(line)
        .map(|escape| {
            let start = escape.start();
            let as_str = escape.as_str();
            let ret = (start - sum_width, as_str);
            sum_width += as_str.len();
            ret
        })
        .collect::<Vec<_>>();

    // The matches of the term you're looking for, so that you can easily determine where
    // the invert attributes will be placed
    let matches = query
        .find_iter(&stripped_str)
        .flat_map(|c| [c.start(), c.end()])
        .collect::<Vec<_>>();

    // Highlight all the instances of the search term in the stripped string
    // by inverting their background/foreground colors
    let mut inverted = query
        .replace_all(&stripped_str, |caps: &regex::Captures| {
            format!("{}{}{}", *INVERT, &caps[0], *NORMAL)
        })
        .to_string();

    // inserted_escs_len == the total length of the ascii escapes which have been re-inserted
    // into the stripped string at the point where it is being checked.
    let mut inserted_escs_len = 0;
    for esc in escapes {
        let match_count = matches.iter().take_while(|m| **m <= esc.0).count();
        // Find how many invert|normal markers appear before this escape

        // find the number of invert strings and number of uninvert strings that have been
        // inserted up to this point in the string
        let num_invert = match_count / 2;
        let num_normal = match_count - num_invert;

        // calculate the index which this escape should be re-inserted at by adding
        // its position in the stripped string to the total length of the ansi escapes
        // (both highlighting and the ones from the original string).
        // TODO: Add more docs to this
        let mut pos = if !accurate && match_count % 2 == 1 {
            // INFO: Its safe to unwrap here
            matches.get(match_count).unwrap()
                + NORMAL.len()
                + inserted_escs_len
                + (num_invert * INVERT.len())
                + (num_normal * NORMAL.len())
        } else {
            esc.0 + inserted_escs_len + (num_invert * INVERT.len()) + (num_normal * NORMAL.len())
        };

        if match_count % 2 == 1 {
            pos = pos.saturating_sub(1);
        }

        // insert the escape back in
        inverted.insert_str(pos, esc.1);

        // increment the length of the escapes inserted back in
        inserted_escs_len += esc.1.len();
    }

    inverted
}

/// Highlights the search match
///
/// The first return value returns the line that has all the search matches highlighted
/// The second tells whether a search match was actually found
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn highlight_line_matches(
    line: &str,
    query: &regex::Regex,
    accurate: bool,
) -> (String, bool) {
    let highlighted = highlight_matches_args(line, query, accurate);
    (highlighted.to_string(), highlighted.is_match)
}

pub(crate) struct HighlightMatchesArgs<'a, 'b> {
    line: &'a str,
    query: &'b Regex,
    accurate: bool,
    is_match: bool,
}

impl fmt::Display for HighlightMatchesArgs<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.is_match {
            return f.write_str(self.line);
        }

        if !ANSI_REGEX.is_match(self.line) {
            let mut last = 0;
            for matched in self.query.find_iter(self.line) {
                f.write_str(&self.line[last..matched.start()])?;
                write!(f, "{}{}{}", *INVERT, matched.as_str(), *NORMAL)?;
                last = matched.end();
            }
            return f.write_str(&self.line[last..]);
        }

        f.write_str(&highlight_line_matches_ansi(
            self.line,
            self.query,
            self.accurate,
        ))
    }
}

/// Returns the position of the nth search match relative to `upper_mark`.
///
/// This function will return the index of the nth search match present in
/// [`PagerState::search_state::search_idx`] relative to `upper_mark`.
/// - If `jump` is a strictly positive value, the returned index will be strictly `jump` matches
///   ahead `upper_mark`.
/// - If `jump` is a strictly negative value, the returned index will be strictly `jump` matches
///   before.`upper_mark`.
/// - If `jump` is 0, the returned index will be at `upper_mark` if it contains a search match or
///   the next match immediately after `upper_mark` (after in this context depends on the direction).
#[must_use]
pub(crate) fn nth_match(
    search_idx: &BTreeSet<usize>,
    upper_mark: usize,
    jump: isize,
    direction: SearchMode,
) -> Option<usize> {
    if search_idx.is_empty() {
        return None;
    }

    let nearest_idx = match (jump, direction) {
        (1.., _) => search_idx.iter().position(|i| *i > upper_mark),
        (0, SearchMode::Forward) => search_idx.iter().position(|i| *i >= upper_mark),
        (0, SearchMode::Reverse) => search_idx.iter().rposition(|i| *i <= upper_mark),
        (..=-1, _) => search_idx.iter().rposition(|i| *i < upper_mark),
        (_, SearchMode::Unknown) => unreachable!(),
    };

    let last_idx = search_idx.len().saturating_sub(1).cast_signed();
    let fallback_idx = match (jump, direction) {
        (1.., _) | (0, SearchMode::Forward) => last_idx,
        (..=-1, _) | (0, SearchMode::Reverse) => 0,
        (_, SearchMode::Unknown) => unreachable!(),
    };
    let mut start_idx = nearest_idx.map_or(fallback_idx, usize::cast_signed);
    if jump > 0 {
        start_idx += jump - 1;
    } else if jump < 0 {
        start_idx += jump + 1;
    }

    start_idx = start_idx.clamp(0, last_idx);

    Some(start_idx.cast_unsigned())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::SearchMode;

    fn search_idx() -> std::collections::BTreeSet<usize> {
        BTreeSet::from([2, 10, 15, 17, 50])
    }

    #[test]
    fn nth_match_returns_none_for_empty_search_index() {
        let search_idx = std::collections::BTreeSet::new();
        assert_eq!(
            super::nth_match(&search_idx, 10, 1, SearchMode::Forward),
            None
        );
    }

    #[test]
    fn nth_match_zero_jump_forward_returns_match_at_upper_mark() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 10, 0, SearchMode::Forward),
            Some(1)
        );
    }

    #[test]
    fn nth_match_zero_jump_forward_returns_next_match_after_upper_mark() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 11, 0, SearchMode::Forward),
            Some(2)
        );
    }

    #[test]
    fn nth_match_zero_jump_reverse_returns_match_at_upper_mark() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 10, 0, SearchMode::Reverse),
            Some(1)
        );
    }

    #[test]
    fn nth_match_zero_jump_reverse_returns_previous_match_before_upper_mark() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 11, 0, SearchMode::Reverse),
            Some(1)
        );
    }

    #[test]
    fn nth_match_positive_jump_moves_strictly_ahead_of_upper_mark() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 10, 1, SearchMode::Forward),
            Some(2)
        );
        assert_eq!(
            super::nth_match(&search_idx, 10, 2, SearchMode::Forward),
            Some(3)
        );
    }

    #[test]
    fn nth_match_negative_jump_moves_strictly_before_upper_mark() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 17, -1, SearchMode::Reverse),
            Some(2)
        );
        assert_eq!(
            super::nth_match(&search_idx, 17, -2, SearchMode::Reverse),
            Some(1)
        );
    }

    #[test]
    fn nth_match_clamps_to_first_match_when_reverse_search_has_no_previous_match() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 1, 0, SearchMode::Reverse),
            Some(0)
        );
        assert_eq!(
            super::nth_match(&search_idx, 1, -1, SearchMode::Reverse),
            Some(0)
        );
    }

    #[test]
    fn nth_match_clamps_to_last_match_when_forward_search_has_no_later_match() {
        let search_idx = search_idx();
        assert_eq!(
            super::nth_match(&search_idx, 100, 0, SearchMode::Forward),
            Some(4)
        );
        assert_eq!(
            super::nth_match(&search_idx, 100, 1, SearchMode::Forward),
            Some(4)
        );
    }

    #[allow(clippy::trivial_regex)]
    mod highlighting {
        use crate::search::{INVERT, NORMAL, highlight_line_matches};
        use crossterm::style::Attribute;
        use regex::Regex;

        // generic escape code
        const ESC: &str = "\x1b[34m";
        const NONE: &str = "\x1b[0m";

        mod consistent {
            use super::*;

            #[test]
            fn test_highlight_matches() {
                let line = "Integer placerat tristique nisl. placerat non mollis, magna orci dolor, placerat at vulputate neque nulla lacinia eros.".to_string();
                let pat = Regex::new(r"\W\w+t\W").unwrap();
                let result = format!(
                    "Integer{inverse} placerat {noinverse}tristique nisl.\
{inverse} placerat {noinverse}non mollis, magna orci dolor,\
{inverse} placerat {noinverse}at vulputate neque nulla lacinia \
eros.",
                    inverse = Attribute::Reverse,
                    noinverse = Attribute::NoReverse
                );

                assert_eq!(highlight_line_matches(&line, &pat, false).0, result);
            }

            #[test]
            fn no_match() {
                let orig = "no match";
                let res = highlight_line_matches(orig, &Regex::new("test").unwrap(), false);
                assert_eq!(res.0, orig.to_string());
            }

            #[test]
            fn single_match_no_esc() {
                let res =
                    highlight_line_matches("this is a test", &Regex::new(" a ").unwrap(), false);
                assert_eq!(res.0, format!("this is{} a {}test", *INVERT, *NORMAL));
            }

            #[test]
            fn multi_match_no_esc() {
                let res = highlight_line_matches(
                    "test another test",
                    &Regex::new("test").unwrap(),
                    false,
                );
                assert_eq!(
                    res.0,
                    format!("{i}test{n} another {i}test{n}", i = *INVERT, n = *NORMAL)
                );
            }

            // NOTE: esc_pair means a single pair of ESC and NONE

            #[test]
            fn esc_pair_outside_match() {
                let res = highlight_line_matches(
                    &format!("{ESC}color{NONE} and test"),
                    &Regex::new("test").unwrap(),
                    false,
                );
                assert_eq!(
                    res.0,
                    format!("{}color{} and {}test{}", ESC, NONE, *INVERT, *NORMAL)
                );
            }

            #[test]
            fn esc_pair_end_in_match() {
                let orig = format!("this {ESC}is a te{NONE}st");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), false);
                assert_eq!(
                    res.0,
                    format!("this {}is a {}test{}{}", ESC, *INVERT, *NORMAL, NONE)
                );
            }

            #[test]
            fn esc_pair_start_in_match() {
                let orig = format!("this is a te{ESC}st again{NONE}");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), false);
                assert_eq!(
                    res.0,
                    format!("this is a {}test{}{ESC} again{}", *INVERT, *NORMAL, NONE)
                );
            }

            #[test]
            fn esc_pair_around_match() {
                let orig = format!("this is {ESC}a test again{NONE}");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), false);
                assert_eq!(
                    res.0,
                    format!("this is {}a {}test{} again{}", ESC, *INVERT, *NORMAL, NONE)
                );
            }

            #[test]
            fn esc_pair_within_match() {
                let orig = format!("this is a t{ESC}es{NONE}t again");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), false);
                assert_eq!(
                    res.0,
                    format!("this is a {}test{}{ESC}{NONE} again", *INVERT, *NORMAL)
                );
            }

            #[test]
            fn multi_escape_match() {
                let orig = format!("this {ESC}is a te{NONE}st again {ESC}yeah{NONE} test");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), false);
                assert_eq!(
                    res.0,
                    format!(
                        "this {e}is a {i}test{n}{nn} again {e}yeah{nn} {i}test{n}",
                        e = ESC,
                        i = *INVERT,
                        n = *NORMAL,
                        nn = NONE
                    )
                );
            }
        }
        mod accurate {
            use super::*;
            #[test]
            fn correct_ascii_sequence_placement() {
                let orig = format!(
                    "{ESC}test{NONE} this {ESC}is a te{NONE}st again {ESC}yeah{NONE} test",
                );

                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), true);
                assert_eq!(
                    res.0,
                    format!(
                        "{i}{e}test{n}{nn} this {e}is a {i}te{NONE}st{n} again {e}yeah{nn} {i}test{n}",
                        e = ESC,
                        i = *INVERT,
                        n = *NORMAL,
                        nn = NONE
                    )
                );
            }

            // NOTE: esc_pair means a single pair of ESC and NONE
            #[test]
            fn esc_pair_outside_match() {
                let res = highlight_line_matches(
                    &format!("{ESC}color{NONE} and test"),
                    &Regex::new("test").unwrap(),
                    true,
                );
                assert_eq!(
                    res.0,
                    format!("{}color{} and {}test{}", ESC, NONE, *INVERT, *NORMAL)
                );
            }

            #[test]
            fn esc_pair_end_in_match() {
                let orig = format!("this {ESC}is a te{NONE}st");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), true);
                assert_eq!(
                    res.0,
                    format!("this {ESC}is a {}te{NONE}st{}", *INVERT, *NORMAL)
                );
            }

            #[test]
            fn esc_pair_start_in_match() {
                let orig = format!("this is a te{ESC}st again{NONE}");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), true);
                assert_eq!(
                    res.0,
                    format!("this is a {}te{ESC}st{} again{NONE}", *INVERT, *NORMAL)
                );
            }

            #[test]
            fn esc_pair_around_match() {
                let orig = format!("this is {ESC}a test again{NONE}");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), true);
                assert_eq!(
                    res.0,
                    format!("this is {ESC}a {}test{} again{NONE}", *INVERT, *NORMAL)
                );
            }

            #[test]
            fn esc_pair_within_match() {
                let orig = format!("this is a t{ESC}es{NONE}t again");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), true);
                assert_eq!(
                    res.0,
                    format!("this is a {}t{ESC}es{NONE}t{} again", *INVERT, *NORMAL)
                );
            }

            #[test]
            fn multi_escape_match() {
                let orig = format!("this {ESC}is a te{NONE}st again {ESC}yeah{NONE} test");
                let res = highlight_line_matches(&orig, &Regex::new("test").unwrap(), true);
                assert_eq!(
                    res.0,
                    format!(
                        "this {e}is a {i}te{nn}st{n} again {e}yeah{nn} {i}test{n}",
                        e = ESC,
                        i = *INVERT,
                        n = *NORMAL,
                        nn = NONE
                    )
                );
            }
        }
    }
}
