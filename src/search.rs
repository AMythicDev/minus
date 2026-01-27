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
//! Running Incremental search can be controlled by a function. The function should take
//! reference to [SearchOpts] as the only argument and return a bool as output. This way we can impose a
//! condition so that incremental search does not get really resource intensive for really vague queries
//! This also allows applications can control whether they want incremental search to run.
//! By default minus uses a default condition where incremental search runs only when length of search
//! query is greater than 1 and number of screen lines (lines obtained after taking care of wrapping,
//! mapped to a single row on the terminal) is greater than 5000.
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
//! pager.set_incremental_search_condition(Box::new(|so: &SearchOpts| so.string.len() > 1)).unwrap();
//! ```
//! To completely disable incremental search, set the condition to false
//! ```
//! use minus::{Pager, search::SearchOpts};
//!
//! let pager = Pager::new();
//! pager.set_incremental_search_condition(Box::new(|_| false)).unwrap();
//! ```
//! Similarly to always run incremental search, set the condition to true
//! ```
//! use minus::{Pager, search::SearchOpts};
//!
//! let pager = Pager::new();
//! pager.set_incremental_search_condition(Box::new(|_| true)).unwrap();
//! ```

use crate::minus_core::utils::{display, term};
use crate::screen::Screen;
use crate::{LineNumbers, PagerState};
use crate::{error::MinusError, screen};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use regex::Regex;
use std::collections::BTreeSet;
use std::{
    convert::{TryFrom, TryInto},
    io::Write,
    sync::LazyLock,
    time::Duration,
};
use unicode_segmentation::UnicodeSegmentation;

static INVERT: LazyLock<String> = LazyLock::new(|| Attribute::Reverse.to_string());
static NORMAL: LazyLock<String> = LazyLock::new(|| Attribute::NoReverse.to_string());
static ANSI_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("[\\u001b\\u009b]\\[[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]")
        .unwrap()
});

#[derive(Clone, Copy, Debug, Default, Eq)]
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
    /// A [crossterm Event](Event) on which to respond
    pub ev: Option<Event>,
    /// Current string query
    pub string: String,
    /// Status of the input prompt. See [InputStatus]
    pub input_status: InputStatus,
    /// Specifies the terminal column number that the cursor on at the prompt site.
    /// It can range between 1 and `string.len() + 1`
    pub cursor_position: u16,
    /// Direction of search. See [SearchMode].
    pub search_mode: SearchMode,
    /// Search character, either `/` or `?` depending on [SearchMode]
    pub search_char: char,
    /// Number of rows available in the terminal
    pub rows: u16,
    /// Number of cols available in the terminal
    pub cols: u16,
    /// Options specifically controlling incremental search
    pub incremental_search_options: Option<IncrementalSearchOpts<'a>>,
    incremental_search_cache: Option<IncrementalSearchCache>,
    compiled_regex: Option<Regex>,
}

/// Options to control incremental search
///
/// NOTE: `text` and `initial_formatted_lines` are experimental in this context and are subject to
/// change. Use them at your own risk.
pub struct IncrementalSearchOpts<'a> {
    /// Current status of line numbering
    pub line_numbers: LineNumbers,
    /// Value of [PagerState::upper_mark] before starting of search prompt
    pub initial_upper_mark: usize,
    /// Reference to [PagerState::screen]
    pub screen: &'a Screen,
    /// Value of [PagerState::upper_mark] before starting of search prompt
    pub initial_left_mark: usize,
}

impl<'a> From<&'a PagerState> for IncrementalSearchOpts<'a> {
    fn from(ps: &'a PagerState) -> Self {
        Self {
            line_numbers: ps.line_numbers,
            initial_upper_mark: ps.upper_mark,
            screen: &ps.screen,
            initial_left_mark: ps.left_mark,
        }
    }
}

#[allow(clippy::fallible_impl_from)]
impl<'a> From<&'a PagerState> for SearchOpts<'a> {
    fn from(ps: &'a PagerState) -> Self {
        let search_char = match ps.search_state.search_mode {
            SearchMode::Forward => '/',
            SearchMode::Reverse => '?',
            SearchMode::Unknown => unreachable!(),
        };

        let incremental_search_options = IncrementalSearchOpts::from(ps);

        Self {
            ev: None,
            string: String::with_capacity(200),
            input_status: InputStatus::Active,
            cursor_position: 0,
            search_char,
            rows: ps.rows.try_into().unwrap(),
            cols: ps.cols.try_into().unwrap(),
            incremental_search_options: Some(incremental_search_options),
            incremental_search_cache: None,
            compiled_regex: None,
            search_mode: ps.search_state.search_mode,
        }
    }
}

/// Status of the search prompt
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum InputStatus {
    /// Closed due to confirmation of search query using `Enter`
    Confirmed,
    /// Closed due to abortion using `Esc`
    Cancelled,
    /// Search prompt is open
    Active,
}

impl InputStatus {
    /// Returns true if the input prompt is closed either by confirming the query or by cancelling
    /// he search
    #[must_use]
    pub const fn done(&self) -> bool {
        matches!(self, Self::Cancelled | Self::Confirmed)
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
    /// Lines to be displayed with highlighted search matches
    pub(crate) formatted_lines: Vec<String>,
    /// Index from `search_idx` where a search match after current upper mark may be found
    /// NOTE: There is no guarantee that this will stay within the bounds of `search_idx`
    pub(crate) search_mark: usize,
    /// Indices of formatted_lines where search matches have been found
    pub(crate) search_idx: BTreeSet<usize>,
    /// Index of the line from which to display the text.
    /// This will be set to the index of line which is after the current upper mark and will
    /// have a search match for sure
    pub(crate) upper_mark: usize,
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
    incremental_search_condition: F,
) -> crate::Result<Option<IncrementalSearchCache>>
where
    O: Write,
    F: Fn(&'a SearchOpts) -> bool,
{
    if so.incremental_search_options.is_none() {
        return Ok(None);
    }
    let iso = so.incremental_search_options.as_ref().unwrap();

    // Check if we can continue forward with incremental search
    let should_proceed = so.compiled_regex.is_some() && incremental_search_condition(so);

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

    // Format the text with search highlights and get the index of the element in
    // format_result.append_search_idx which is after the current upper mark
    //
    // PERF: Check if this can be futhur optimized
    let (buffer, format_result) = screen::make_format_lines(
        &iso.screen.orig_text,
        iso.line_numbers,
        so.cols.into(),
        iso.screen.line_wrapping,
        so.compiled_regex.as_ref(),
    );
    let position_of_next_match =
        next_nth_match(&format_result.append_search_idx, iso.initial_upper_mark, 0);
    // Get the upper mark. If we can't find one, reset the display
    let upper_mark;
    if let Some(pnm) = position_of_next_match {
        upper_mark = *format_result.append_search_idx.iter().nth(pnm).unwrap();
        // Draw the incrementally searched lines from upper mark
        display::write_text_checked(
            out,
            &buffer,
            upper_mark,
            so.rows.into(),
            so.cols.into(),
            iso.screen.line_wrapping,
            iso.initial_left_mark,
            iso.line_numbers,
            iso.screen.line_count(),
        )?;
    } else {
        reset_screen(out, so)?;
        return Ok(None);
    }
    // Return the results obtained by running incremental search so that they can be stored as a
    // cache.
    Ok(Some(IncrementalSearchCache {
        formatted_lines: buffer,
        search_mark: position_of_next_match.unwrap(),
        upper_mark,
        search_idx: format_result.append_search_idx,
    }))
}

/// Respond to keyboard events
///
/// This souuld be called exactly once for each event by [fetch_input]
#[expect(clippy::too_many_lines)]
fn handle_key_press<O, F>(
    out: &mut O,
    so: &mut SearchOpts<'_>,
    incremental_search_condition: F,
) -> crate::Result
where
    O: Write,
    F: Fn(&SearchOpts<'_>) -> bool,
{
    // If no event is present, abort
    if so.ev.is_none() {
        return Ok(());
    }

    let refresh_display = |out: &mut O, so: &mut SearchOpts<'_>| -> Result<(), MinusError> {
        // Cache the compiled regex if the regex is valid
        so.compiled_regex = Regex::new(&so.string).ok();

        // Run incremental search and update the upper mark if incremental search had a successful
        // run otherwise set it to the initial upper mark
        so.incremental_search_cache =
            run_incremental_search(out, so, incremental_search_condition)?;

        // Update prompt
        term::move_cursor(out, 0, so.rows, false)?;
        write!(
            out,
            "\r{}{}{}",
            Clear(ClearType::CurrentLine),
            so.search_char,
            so.string,
        )?;
        Ok(())
    };

    match so.ev.as_ref().unwrap() {
        Event::Key(KeyEvent { kind, .. }) if *kind != KeyEventKind::Press => return Ok(()),
        // If Esc is pressed, cancel the search and also make sure that the search query is
        // ")cleared
        Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.string.clear();
            so.input_status = InputStatus::Cancelled;
            return Ok(());
        }
        Event::Key(KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            // On backspace, remove the last character just before the cursor from the so.string
            // But if we are at very first character, do nothing.
            if so.cursor_position == 0 {
                return Ok(());
            }

            let max_idx = so.string.graphemes(true).count();
            let cursor_pos = usize::from(so.cursor_position);
            if let Some((idx, s)) = so
                .string
                .grapheme_indices(true)
                .nth_back(max_idx - cursor_pos)
            {
                let num_chars = s.chars().count();
                for _ in 0..num_chars {
                    so.string.remove(idx);
                }
            }

            so.cursor_position = so.cursor_position.saturating_sub(1);
            // Update the line
            refresh_display(out, so)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Delete,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            let max_idx = so.string.graphemes(true).count();
            let cursor_pos = usize::from(so.cursor_position);

            // On delete, remove the character under the cursor from the so.string
            // But if we are at the column right after the last character, do nothing.
            if cursor_pos >= max_idx {
                return Ok(());
            }

            // we want to do cursor_pos + 1 'cause we're handling the delete key, not backspace. So
            // we won't remove any characters if you're at the very end of the string.
            if let Some(idx_back) = max_idx.checked_sub(cursor_pos + 1)
            // we want to do nth_back since it's much more likely that someone's cursor will be at
            // the end of the string instead of the front
                && let Some((idx, _)) = so.string.char_indices().nth_back(idx_back)
            {
                so.string.remove(idx);
            }

            // Update the line
            refresh_display(out, so)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.input_status = InputStatus::Confirmed;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            if so.cursor_position == 0 {
                return Ok(());
            }
            so.cursor_position = so.cursor_position.saturating_sub(1);
        }
        Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // here, we're going through the words, and accumulating how many graphemes they take
            // up. Once we reach the word where our cursor currently resides, set our cursor to the
            // beginning of the word's grapheme count and stop iterating.
            let mut acc = 0;
            for s in so.string.split_word_bounds() {
                let graphemes = s.graphemes(true).count();
                if acc + graphemes >= usize::from(so.cursor_position) {
                    so.cursor_position = u16::try_from(acc).unwrap();
                    break;
                }

                acc += graphemes;
            }
        }
        Event::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            let last_available_idx = so.string.graphemes(true).count();
            if usize::from(so.cursor_position) >= last_available_idx {
                return Ok(());
            }

            so.cursor_position = so.cursor_position.saturating_add(1);
        }
        Event::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // here, we're going through the words, and accumulating how many graphemes they take
            // up. Once we reach the word where our cursor currently resides, set our cursor to the
            // end of the word's grapheme count and stop iterating.
            let mut acc = 0;
            for s in so.string.split_word_bounds() {
                acc += s.graphemes(true).count();
                if acc > usize::from(so.cursor_position) {
                    so.cursor_position = u16::try_from(acc).unwrap();
                    break;
                }
            }
        }
        Event::Key(KeyEvent {
            code: KeyCode::Home,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.cursor_position = 0;
        }
        Event::Key(KeyEvent {
            code: KeyCode::End,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.cursor_position = so.string.graphemes(true).count().try_into().unwrap();
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            // For any character key, without a modifier, insert it into so.string before
            // current cursor position and update the line

            let orig_num_graphemes = so.string.graphemes(true).count();

            let c = *c;
            let insert_byte_idx = so
                .string
                .grapheme_indices(true)
                .nth(usize::from(so.cursor_position))
                .map_or_else(|| so.string.len(), |(idx, _)| idx);

            so.string.insert(insert_byte_idx, c);
            let new_num_graphemes = so.string.graphemes(true).count();
            println!(
                "orig was {orig_num_graphemes}, but new is {new_num_graphemes}. cursor is {}",
                so.cursor_position
            );
            so.cursor_position += u16::try_from(new_num_graphemes - orig_num_graphemes).unwrap();

            // This won't panic 'cause it's guaranteed to return 1..=4. Don't know why it returns a
            // usize instead of a u8, though.
            // populate_word_index(so);
            refresh_display(out, so)?;
        }
        _ => return Ok(()),
    }

    term::move_cursor(out, so.cursor_position + 1, so.rows, false)?;
    out.flush().map_err(MinusError::from)
}

/// Fetch the search query
///
/// The function will change the prompt to `/` for Forward search or `?` for Reverse search.
/// Next it fetches and handles all events from the terminal screen until [SearchOpts::input_status] isn't
/// set to either [InputStatus::Cancelled] or [InputStatus::Confirmed] by pressing `Esc` or
/// `Enter` respectively.
/// Finally we return
pub(crate) fn fetch_input(
    out: &mut impl std::io::Write,
    ps: &PagerState,
) -> Result<FetchInputResult, MinusError> {
    // Set the search character to show at column 0
    let search_char = match ps.search_state.search_mode {
        SearchMode::Forward => '/',
        SearchMode::Unknown | SearchMode::Reverse => '?',
    };

    // Initial setup
    // - Place the cursor at the beginning of prompt line
    // - Clear the prompt
    // - Write the search character and
    // - Show the cursor
    term::move_cursor(out, 0, ps.rows.try_into().unwrap(), false)?;
    write!(
        out,
        "{}{}{}",
        Clear(ClearType::CurrentLine),
        search_char,
        cursor::Show
    )?;
    out.flush()?;

    let mut search_opts = SearchOpts::from(ps);

    // Fetch events from the terminal and handle them
    loop {
        if event::poll(Duration::from_millis(100)).map_err(|e| MinusError::HandleEvent(e.into()))? {
            let ev = event::read().map_err(|e| MinusError::HandleEvent(e.into()))?;
            search_opts.ev = Some(ev);
            handle_key_press(
                out,
                &mut search_opts,
                &ps.search_state.incremental_search_condition,
            )?;
            search_opts.ev = None;
        }
        if search_opts.input_status.done() {
            break;
        }
    }
    // Teardown: almost opposite of setup
    term::move_cursor(out, 0, ps.rows.try_into().unwrap(), false)?;
    write!(out, "{}{}", Clear(ClearType::CurrentLine), cursor::Hide)?;
    out.flush()?;

    let fetch_input_result = match search_opts.input_status {
        InputStatus::Active => unreachable!(),
        InputStatus::Cancelled => FetchInputResult::new_empty(),
        // When the query is confirmed, return the actual query along with everything that is valid
        // in the cache
        InputStatus::Confirmed => FetchInputResult {
            string: search_opts.string,
            incremental_search_result: search_opts.incremental_search_cache,
            compiled_regex: search_opts.compiled_regex,
        },
    };
    Ok(fetch_input_result)
}

/// Highlights the search match
///
/// The first return value returns the line that has all the search matches highlighted
/// The second tells whether a search match was actually found
pub(crate) fn highlight_line_matches(
    line: &str,
    query: &regex::Regex,
    accurate: bool,
) -> (String, bool) {
    // Remove all ansi escapes so we can look through it as if it had none
    let stripped_str = ANSI_REGEX.replace_all(line, "");

    // if it doesn't match, don't even try. Just return.
    if !query.is_match(&stripped_str) {
        return (line.to_string(), false);
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

    (inverted, true)
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
/// to the index of last element in `search_idx`.Also if search_idx is empty, this will simply
/// return None.
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
    // One we find that, we add n-1 to it to get the next nth match after upper_mark
    let mut position_of_next_match;
    if let Some(nearest_idx) = search_idx.iter().position(|i| {
        if jump == 0 {
            *i >= upper_mark
        } else {
            *i > upper_mark
        }
    }) {
        // This ensures that index doesn't get off-by-one in case of jump = 0
        if jump == 0 {
            position_of_next_match = nearest_idx;
        } else {
            position_of_next_match = nearest_idx.saturating_add(jump).saturating_sub(1);
        }

        // If position_of_next_match is goes beyond the length of search_idx
        // set it to the length of search_idx -1 which corresponds to the index of
        // last match
        if position_of_next_match > search_idx.len().saturating_sub(1) {
            position_of_next_match = search_idx.len().saturating_sub(1);
        }
    } else {
        // If there's no match at all simply set it to the length of search_idx -1 which
        // corresponds to the index of last match
        position_of_next_match = search_idx.len().saturating_sub(1);
    }

    Some(position_of_next_match)
}

#[cfg(test)]
mod tests {
    mod input_handling {
        use crate::{
            SearchMode,
            search::{InputStatus, SearchOpts, handle_key_press},
        };
        use crossterm::{
            cursor::MoveTo,
            event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
            terminal::{Clear, ClearType},
        };
        use pretty_assertions::{assert_eq, assert_str_eq};
        use std::{convert::TryInto, io::Write};
        use unicode_segmentation::UnicodeSegmentation;

        fn new_search_opts(sm: SearchMode) -> SearchOpts<'static> {
            let search_char = match sm {
                SearchMode::Forward => '/',
                SearchMode::Reverse => '?',
                SearchMode::Unknown => unreachable!(),
            };

            SearchOpts {
                ev: None,
                string: String::with_capacity(200),
                input_status: InputStatus::Active,
                cursor_position: 0,
                search_char,
                rows: 25,
                cols: 100,
                incremental_search_options: None,
                incremental_search_cache: None,
                compiled_regex: None,
                search_mode: sm,
            }
        }

        const fn make_event_from_keycode(kc: KeyCode) -> Event {
            Event::Key(KeyEvent {
                code: kc,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::NONE,
                state: KeyEventState::NONE,
            })
        }

        const QUERY_STRING: &str =
            "this is@complex-text_seärch?query🤣with\u{8205} 🖐🏼complex emojis";
        const EXPECTED_WORD_INDICES: [u16; 18] = [
            0, 4, 5, 7, 8, 15, 16, 27, 28, 33, 34, 38, 39, 40, 41, 48, 49, 55,
        ];

        fn pretest_setup_forward_search() -> (SearchOpts<'static>, Vec<u8>, u16, &'static str) {
            let last_movable_column = u16::try_from(QUERY_STRING.graphemes(true).count()).unwrap();

            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);

            for (i, c) in QUERY_STRING.graphemes(true).enumerate() {
                for c in c.chars() {
                    press_key(&mut out, &mut search_opts, KeyCode::Char(c));
                }
                assert_eq!(usize::from(search_opts.cursor_position), i + 1);
            }
            assert_eq!(search_opts.cursor_position, last_movable_column);
            (search_opts, out, last_movable_column, QUERY_STRING)
        }

        fn press_key(out: &mut Vec<u8>, so: &mut SearchOpts<'_>, code: KeyCode) {
            so.ev = Some(make_event_from_keycode(code));
            handle_key_press(out, so, |_| false).unwrap();
        }

        #[test]
        fn input_sequential_text() {
            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);
            for (i, c) in "text search matches".graphemes(true).enumerate() {
                for c in c.chars() {
                    press_key(&mut out, &mut search_opts, KeyCode::Char(c));
                }
                assert_eq!(search_opts.input_status, InputStatus::Active);
                assert_eq!(search_opts.cursor_position as usize, i + 1);
            }
            press_key(&mut out, &mut search_opts, KeyCode::Enter);
            // assert_eq!(search_opts.word_index, vec![0, 4, 5, 11, 12]);
            assert_eq!(&search_opts.string, "text search matches");
            assert_eq!(search_opts.input_status, InputStatus::Confirmed);
        }

        #[test]
        fn input_complex_sequential_text() {
            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);
            for (i, c) in QUERY_STRING.graphemes(true).enumerate() {
                for c in c.chars() {
                    press_key(&mut out, &mut search_opts, KeyCode::Char(c));
                }
                assert_eq!(search_opts.input_status, InputStatus::Active);
                assert_eq!(search_opts.cursor_position as usize, i + 1);
            }

            press_key(&mut out, &mut search_opts, KeyCode::Enter);
            assert_eq!(&search_opts.string, QUERY_STRING);
            assert_eq!(search_opts.input_status, InputStatus::Confirmed);
        }

        #[test]
        fn home_end_keys() {
            // Setup
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup_forward_search();

            press_key(&mut out, &mut search_opts, KeyCode::Home);
            assert_eq!(search_opts.cursor_position as usize, 0);

            press_key(&mut out, &mut search_opts, KeyCode::End);
            assert_eq!(search_opts.cursor_position, last_movable_column);
        }

        #[test]
        fn basic_left_arrow_movement() {
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup_forward_search();

            // We are currently at the very next column to the last char

            // Check functionality of left arrow key
            // Pressing left arrow moves the cursor towards the beginning of string until it
            // reaches the first char after which pressing it further would not have any effect
            for i in (0..last_movable_column).rev() {
                press_key(&mut out, &mut search_opts, KeyCode::Left);
                assert_eq!(search_opts.cursor_position, i);
            }
            // Pressing Left arrow any more will not make any effect
            press_key(&mut out, &mut search_opts, KeyCode::Left);
            assert_eq!(search_opts.cursor_position, 0);
        }

        #[test]
        fn basic_right_arrow_movement() {
            // Setup
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup_forward_search();
            // Go to the 1st char
            press_key(&mut out, &mut search_opts, KeyCode::Home);

            // Check functionality of right arrow key
            // Pressing right arrow moves the cursor towards the end of string until it
            // reaches the very next column to the last char after which pressing it further would not have any effect
            for i in 1..=last_movable_column {
                press_key(&mut out, &mut search_opts, KeyCode::Right);
                assert_eq!(search_opts.cursor_position, i);
            }
            // Pressing right arrow any more will not make any effect
            press_key(&mut out, &mut search_opts, KeyCode::Right);
            assert_eq!(search_opts.cursor_position, last_movable_column);
        }

        #[test]
        fn right_jump_by_word() {
            // Setup
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup_forward_search();
            // let jump_columns: [u16; 10] = [0, 4, 5, 7, 8, 15, 16, 27, 28, last_movable_column];

            // Go to the 1st char
            press_key(&mut out, &mut search_opts, KeyCode::Home);

            let ev = Event::Key(KeyEvent {
                code: KeyCode::Right,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::CONTROL,
                state: KeyEventState::NONE,
            });

            // Jump right word by word
            for i in &EXPECTED_WORD_INDICES[1..] {
                search_opts.ev = Some(ev.clone());
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.cursor_position, *i);
            }
            // Pressing ctrl+right will not do anything any keep the cursor at the very next column
            // to the last char
            search_opts.ev = Some(ev);
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position, last_movable_column);
        }

        #[test]
        fn left_jump_by_word() {
            // Setup
            let (mut search_opts, mut out, _, _) = pretest_setup_forward_search();

            // We are currently at the very next column to the last char
            let ev = Event::Key(KeyEvent {
                code: KeyCode::Left,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::CONTROL,
                state: KeyEventState::NONE,
            });

            // Jump right word by word
            for i in (EXPECTED_WORD_INDICES[..(EXPECTED_WORD_INDICES.len() - 1)])
                .iter()
                .rev()
            {
                search_opts.ev = Some(ev.clone());
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.cursor_position, *i);
            }

            // Pressing ctrl+left will not do anything and keep the cursor at the very first column
            search_opts.ev = Some(ev);
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position, EXPECTED_WORD_INDICES[0]);
        }

        #[test]
        fn esc_key() {
            let (mut search_opts, mut out, _, _) = pretest_setup_forward_search();

            press_key(&mut out, &mut search_opts, KeyCode::Esc);
            assert_eq!(search_opts.input_status, InputStatus::Cancelled);
        }

        #[test]
        fn forward_sequential_text_input_screen_data() {
            let (search_opts, out, _last_movable_column, query_string) =
                pretest_setup_forward_search();

            let mut result_out = Vec::with_capacity(1500);

            // Try to recreate the behaviour of handle_key_press when new char is entered
            let mut string = String::with_capacity(query_string.len());
            let mut cursor_position: u16;
            for c in query_string.chars() {
                string.push(c);
                cursor_position = u16::try_from(string.graphemes(true).count()).unwrap();
                write!(
                    result_out,
                    "{move_to_prompt}\r{clear_line}/{string}{move_to_position}",
                    move_to_prompt = MoveTo(0, search_opts.rows),
                    clear_line = Clear(ClearType::CurrentLine),
                    move_to_position = MoveTo(cursor_position + 1, search_opts.rows),
                )
                .unwrap();
            }
            assert_eq!(out, result_out);
        }

        #[test]
        fn backward_sequential_text_input_screen_data() {
            let last_movable_column: u16 = QUERY_STRING.graphemes(true).count().try_into().unwrap();

            let mut search_opts = new_search_opts(SearchMode::Reverse);
            let mut out = Vec::with_capacity(1500);

            for c in QUERY_STRING.chars() {
                press_key(&mut out, &mut search_opts, KeyCode::Char(c));
            }
            assert_eq!(search_opts.cursor_position, last_movable_column);

            let mut result_out = Vec::with_capacity(1500);

            // Try to recreate the behaviour of handle_key_press when new char is entered
            let mut string = String::with_capacity(QUERY_STRING.len());
            let mut cursor_position: u16;
            for c in QUERY_STRING.chars() {
                string.push(c);
                cursor_position = u16::try_from(string.graphemes(true).count()).unwrap();
                write!(
                    result_out,
                    "{move_to_prompt}\r{clear_line}?{string}{move_to_position}",
                    move_to_prompt = MoveTo(0, search_opts.rows),
                    clear_line = Clear(ClearType::CurrentLine),
                    move_to_position = MoveTo(cursor_position + 1, search_opts.rows),
                )
                .unwrap();
            }
            assert_eq!(out, result_out);
        }

        #[test]
        fn backspace_while_moving_right() {
            let (mut so, mut out, _, _) = pretest_setup_forward_search();

            press_key(&mut out, &mut so, KeyCode::Home);

            let orig_graphemes = QUERY_STRING.graphemes(true).count();
            for i in 0..orig_graphemes {
                press_key(&mut out, &mut so, KeyCode::Right);
                assert_eq!(so.cursor_position, 1);

                press_key(&mut out, &mut so, KeyCode::Backspace);
                assert_eq!(so.string.graphemes(true).count(), orig_graphemes - (i + 1));
            }

            assert_eq!(so.cursor_position, 0);
            assert_eq!(so.string, "");
        }

        #[test]
        fn backspace_every_other_going_backwards() {
            let (mut so, mut out, _, _) = pretest_setup_forward_search();

            let mut graphemes = QUERY_STRING.graphemes(true).count();
            let mut cursor = u16::try_from(graphemes).unwrap();
            for i in (0..graphemes).rev() {
                if i % 2 == 0 {
                    press_key(&mut out, &mut so, KeyCode::Left);
                    cursor -= 1;
                } else {
                    press_key(&mut out, &mut so, KeyCode::Backspace);
                    cursor -= 1;
                    graphemes -= 1;
                }

                assert_eq!(so.cursor_position, cursor);
                assert_eq!(so.string.graphemes(true).count(), graphemes);
                let expected_str = QUERY_STRING
                    .graphemes(true)
                    .enumerate()
                    .filter(|(g_idx, _)| *g_idx < i || (g_idx % 2 == 0))
                    .map(|(_, g)| g)
                    .collect::<String>();

                assert_str_eq!(so.string, expected_str);
            }
        }

        #[test]
        fn inserting_char_while_not_at_end_keeps_cursor_position() {
            let (mut so, mut out, _, _) = pretest_setup_forward_search();
            let mut current_pos = so.cursor_position;

            for _ in 0..10 {
                press_key(&mut out, &mut so, KeyCode::Left);
            }
            current_pos -= 10;
            assert_eq!(so.cursor_position, current_pos);

            press_key(&mut out, &mut so, KeyCode::Char('!'));
            current_pos += 1;
            assert_eq!(so.cursor_position, current_pos);

            for _ in 0..4 {
                press_key(&mut out, &mut so, KeyCode::Right);
            }
            current_pos += 4;
            assert_eq!(so.cursor_position, current_pos);

            press_key(&mut out, &mut so, KeyCode::Char('a'));
            current_pos += 1;
            assert_eq!(so.cursor_position, current_pos);

            // the cursor position shouldn't change if we then place a combining umlaut after an a,
            // since they're now considered one grapheme.
            press_key(&mut out, &mut so, KeyCode::Char('\u{0308}'));
            assert_eq!(so.cursor_position, current_pos);
        }
    }

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

    #[expect(clippy::trivial_regex)]
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
                let orig = format!("this {ESC}is a te{NONE}st again {ESC}yeah{NONE} test",);
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
                let orig = format!("this {ESC}is a te{NONE}st again {ESC}yeah{NONE} test",);
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
