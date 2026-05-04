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
//! to [SearchOpts] and `&str` containing the currently entered query as arguments and return a bool
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
use crate::{error::MinusError, screen};
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
/// Although it isn't much important for most use cases but it alongside [IncrementalSearchOpts] are the key components
/// when applications want to customize the incremental seaech condition.
///
/// Most of the fields have self-explanatory names so it should be very easy to get started using
/// this
#[allow(clippy::module_name_repetitions)]
pub struct SearchOpts<'a> {
    /// Direction of search. See [SearchMode].
    pub search_mode: SearchMode,
    /// Number of rows available in the terminal
    pub rows: u16,
    /// Number of cols available in the terminal
    pub cols: u16,
    /// Options specifically controlling incremental search
    pub incremental_search_options: Option<IncrementalSearchOpts<'a>>,
    pub(crate) incremental_search_cache: Option<IncrementalSearchCache>,
    pub(crate) compiled_regex: Option<Regex>,
}

/// Options to control incremental search
pub struct IncrementalSearchOpts<'a> {
    /// Current status of line numbering
    pub line_numbers: LineNumbers,
    /// Value of [PagerState::upper_mark] before starting of search prompt
    pub initial_upper_mark: usize,
    /// Reference to [PagerState::screen]
    pub screen: &'a Screen,
    /// Cached map from logical lines to formatted rows.
    pub lines_to_row_map: &'a LinesRowMap,
    /// Value of [PagerState::upper_mark] before starting of search prompt
    pub initial_left_mark: usize,
}

impl<'a> From<&'a PagerState> for IncrementalSearchOpts<'a> {
    fn from(ps: &'a PagerState) -> Self {
        Self {
            line_numbers: ps.line_numbers,
            initial_upper_mark: ps.upper_mark,
            screen: &ps.screen,
            lines_to_row_map: &ps.lines_to_row_map,
            initial_left_mark: ps.left_mark,
        }
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
            incremental_search_cache: None,
            compiled_regex: None,
            search_mode: ps.search_state.search_mode,
        }
    }
}

/// Return type of [fetch_input]
pub(crate) struct FetchInputResult {
    /// Original search query
    pub(crate) string: String,
    /// Incremental search cache if available
    pub(crate) incremental_search_result: Option<IncrementalSearchCache>,
    /// Cached pre-compiled [Regex] if available
    pub(crate) compiled_regex: Option<Regex>,
}

impl FetchInputResult {
    /// Create an empty `FetchInputResult` with string set to empty string and
    /// incremental_search_cache and compiled_regex set to `None`.
    const fn new_empty() -> Self {
        Self {
            string: String::new(),
            incremental_search_result: None,
            compiled_regex: None,
        }
    }
}

/// A cache for storing all the new data obtained by running incremental search
pub(crate) struct IncrementalSearchCache {
    /// Indices of formatted_lines where search matches have been found
    pub(crate) search_idx: BTreeSet<usize>,
    /// Index of the line from which to display the text.
    /// This will be set to the index of line which is after the current upper mark and will
    /// have a search match for sure
    pub(crate) upper_mark: usize,
}

fn line_matches_query(line: &str, query: &Regex) -> bool {
    let stripped = ANSI_REGEX.replace_all(line, "");
    query.is_match(stripped.as_ref())
}

fn incremental_preview(
    iso: &IncrementalSearchOpts<'_>,
    query: &Regex,
    cols: usize,
    rows: usize,
) -> Option<(Vec<String>, usize)> {
    fn preview_line(
        iso: &IncrementalSearchOpts<'_>,
        query: &Regex,
        cols: usize,
        line_number_digits: usize,
        line_idx: usize,
        line: &str,
        visible_lines: &mut Vec<String>,
        upper_mark: &mut Option<usize>,
        writable_rows: usize,
        wrapped: bool,
    ) -> Option<()> {
        // Skip all lines that don't have any match
        if upper_mark.is_none() && !line_matches_query(line, query) {
            return Some(());
        }

        let row_start = *iso.lines_to_row_map.get(line_idx).unwrap_or(&0);
        let mut search_idx = BTreeSet::new();
        let mut formatted_rows = screen::formatted_line(
            line,
            line_number_digits,
            line_idx,
            iso.line_numbers,
            cols,
            iso.screen.line_wrapping,
            row_start,
            &mut search_idx,
            Some(query),
        );

        if upper_mark.is_none() {
            let match_row = *search_idx
                .iter()
                .find(|idx| wrapped || **idx >= iso.initial_upper_mark)?;
            let skip_rows = match_row.saturating_sub(row_start);
            *upper_mark = Some(match_row);
            visible_lines.extend(formatted_rows.drain(skip_rows..));
        } else {
            visible_lines.append(&mut formatted_rows);
        }

        if visible_lines.len() >= writable_rows {
            visible_lines.truncate(writable_rows);
        }

        Some(())
    }

    let writable_rows = rows.saturating_sub(1);
    if writable_rows == 0 {
        return None;
    }

    let start_line_idx = iso.lines_to_row_map.row_to_line(iso.initial_upper_mark)?;
    let line_number_digits = crate::minus_core::utils::digits(iso.screen.line_count());
    let mut visible_lines = Vec::with_capacity(writable_rows);
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
            cols,
            line_number_digits,
            line_idx,
            line,
            &mut visible_lines,
            &mut upper_mark,
            writable_rows,
            false,
        )?;
        if visible_lines.len() >= writable_rows {
            break;
        }
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
                cols,
                line_number_digits,
                line_idx,
                line,
                &mut visible_lines,
                &mut upper_mark,
                writable_rows,
                true,
            )?;
            if visible_lines.len() >= writable_rows {
                break;
            }
        }
    }

    upper_mark.map(|upper_mark| (visible_lines, upper_mark))
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
) -> crate::Result<Option<IncrementalSearchCache>>
where
    O: Write,
    F: Fn(&'a SearchOpts, &'a str) -> bool,
{
    if so.incremental_search_options.is_none() {
        return Ok(None);
    }
    let iso = so.incremental_search_options.as_ref().unwrap();

    // Check if we can continue forward with incremental search
    let should_proceed = so.compiled_regex.is_some() && incremental_search_condition(so, line);

    // **Screen resetting**:
    // This is an important bit when running incremental search.It reset the terminal screen to
    // display the lines from the same location and in the same way as before the search even
    // started. Basically print it exactly how it looked before pressing `/` or `?`,
    let reset_screen = |out: &mut O, so: &SearchOpts<'_>| -> crate::Result {
        display::write_text_checked(
            out,
            &iso.screen.formatted_lines,
            iso.initial_upper_mark,
            so.rows.into(),
            so.cols.into(),
            iso.screen.line_wrapping,
            iso.initial_left_mark,
            iso.line_numbers,
            iso.screen.line_count(),
        )?;
        Ok(())
    };

    // If the query prior to the current one had a successful incremental search run and now the
    // current query isn't a valid regex or the incremental search condition has returned false
    // then
    if so.incremental_search_cache.is_some() && !should_proceed {
        reset_screen(out, so)?;
        return Ok(None);
    }

    // Return immediately if search query isn't valid or incremental search condition is false
    // NOTE: This must come after the reset screen display code in above statement, otherwise this
    // will cover all the cases of the above statement's condition and hence the terminal will ever
    // get reset
    if !should_proceed {
        return Ok(None);
    }

    let query = so.compiled_regex.as_ref().unwrap();

    let Some((visible_lines, upper_mark)) =
        incremental_preview(iso, query, so.cols.into(), so.rows.into())
    else {
        reset_screen(out, so)?;
        return Ok(None);
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

    // Return the results obtained by running incremental search so that they can be stored as a
    // cache.
    let mut search_idx = BTreeSet::new();
    search_idx.insert(upper_mark);
    Ok(Some(IncrementalSearchCache {
        upper_mark,
        search_idx,
    }))
}

// HACK: GET the bare `Write` trait to be `Send` + `Sync` without leaving the lock
struct ThreadSafeWriter<'a>(*mut (dyn Write + 'a));

unsafe impl<'a> Send for ThreadSafeWriter<'a> {}
unsafe impl<'a> Sync for ThreadSafeWriter<'a> {}

impl<'a> Write for ThreadSafeWriter<'a> {
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

impl<'a> Helper for SearchHelper<'a> {}

impl<'a> Highlighter for SearchHelper<'a> {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        let mut out = self.out.lock().unwrap();
        let mut so = self.search_opts.lock().unwrap();

        so.compiled_regex = Regex::new(line).ok();

        if let Ok(Some(cache)) =
            run_incremental_search(&mut *out, &*so, line, self.incremental_search_condition)
        {
            so.incremental_search_cache = Some(cache);
        }

        let _ = term::move_cursor(&mut *out, 0, so.rows, false);
        let _ = out.flush();

        Cow::Borrowed(line)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: CmdKind) -> bool {
        true
    }
}

impl<'a> Validator for SearchHelper<'a> {
    fn validate(&self, _ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
    fn validate_while_typing(&self) -> bool {
        false
    }
}

impl<'a> Hinter for SearchHelper<'a> {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl<'a> Completer for SearchHelper<'a> {
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
    let writer_ptr = out as *mut dyn std::io::Write;
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
                incremental_search_result: so.incremental_search_cache.take(),
                string: str,
            })
        }
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
            let mut out_lock = helper.out.lock().unwrap();
            let so = helper.search_opts.lock().unwrap();
            if let Some(iso) = &so.incremental_search_options {
                let _ = display::write_text_checked(
                    &mut *out_lock,
                    &iso.screen.formatted_lines,
                    iso.initial_upper_mark,
                    so.rows.into(),
                    so.cols.into(),
                    iso.screen.line_wrapping,
                    iso.initial_left_mark,
                    iso.line_numbers,
                    iso.screen.line_count(),
                );
            }
            Ok(FetchInputResult::new_empty())
        }
        Err(ReadlineError::Io(e)) => Err(MinusError::from(e)),
        Err(ReadlineError::Errno(_)) | Err(ReadlineError::Signal(_)) => todo!(),
        Err(_) => Ok(FetchInputResult::new_empty()),
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

/// Return a index of an element from `search_idx` that will contain a search match and
/// will be after the `upper_mark`
///
/// `jump` denotes how many indexes to jump through. For example if `search_idx` is
/// `[5, 17, 25, 34, 42]` and `upper_mark` is at 7 and `jump` is set to 1 then this will
/// return `Some(1)` which is the index of 17. If `n `is set to 3 it will return
/// `Some(3)` which is index of 34.
///
/// If `jump` causes the index to overflow the length of the `search_idx`, the function will set it
/// to wrap to the start of `search_idx`. Also if search_idx is empty, this will simply return None.
///
/// Setting `jump` equal to 0 causes a slight change in behaviour: it will also return the index of
/// element if that element is equal to the current upper mark. In the above example lets say that
/// `upper_mark` is at 17 and `jump` is set to 0 then this will return `Some(1)` as the
/// `upper_mark` and element at index  are equal i.e 17.
#[must_use]
pub(crate) fn next_nth_match(
    search_idx: &BTreeSet<usize>,
    upper_mark: usize,
    jump: usize,
) -> Option<usize> {
    if search_idx.is_empty() {
        return None;
    }

    // Find the index of the match that's exactly after the upper_mark.
    // If there isn't one, wrap to the first match in the file.
    let nearest_idx = search_idx.iter().position(|i| {
        if jump == 0 {
            *i >= upper_mark
        } else {
            *i > upper_mark
        }
    });

    let start_idx = nearest_idx.unwrap_or(0);
    let position_of_next_match = if jump == 0 {
        start_idx
    } else {
        start_idx.saturating_add(jump).saturating_sub(1) % search_idx.len()
    };

    Some(position_of_next_match)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_next_match() {
        // A sample index for mocking actual search index matches
        let search_idx = std::collections::BTreeSet::from([2, 10, 15, 17, 50]);
        let mut upper_mark = 0;
        let mut search_mark;
        for (i, v) in search_idx.iter().enumerate() {
            search_mark = super::next_nth_match(&search_idx, upper_mark, 1);
            assert_eq!(search_mark, Some(i));
            let next_upper_mark = *search_idx.iter().nth(search_mark.unwrap()).unwrap();
            assert_eq!(next_upper_mark, *v);
            upper_mark = next_upper_mark;
        }
    }

    #[test]
    fn test_next_match_wraps_to_top() {
        let search_idx = std::collections::BTreeSet::from([2, 10, 15, 17, 50]);

        assert_eq!(super::next_nth_match(&search_idx, 60, 1), Some(0));
        assert_eq!(super::next_nth_match(&search_idx, 60, 3), Some(2));
        assert_eq!(super::next_nth_match(&search_idx, 50, 1), Some(0));
        assert_eq!(super::next_nth_match(&search_idx, 50, 0), Some(4));
    }

    #[allow(clippy::trivial_regex)]
    mod highlighting {
        use std::collections::BTreeSet;

        use crate::PagerState;
        use crate::search::{INVERT, NORMAL, highlight_line_matches, next_nth_match};
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
