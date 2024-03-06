#![cfg_attr(docsrs, doc(cfg(feature = "search")))]
//! Text searching functionality
//!
//! Text searching inside minus is quite advanced than other terminal pagers. It is highly
//! inspired by modern text editors and hence provides features like:-
//! - [Keybindings](../index.html#key-bindings-available-at-search-prompt) similar to modern text editors
//! - Incremental search
//! - Full regex support for writing advanced search queries
//! and more...
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

#![allow(unused_imports)]
use crate::minus_core::utils::{display, term};
use crate::screen::Screen;
use crate::{error::MinusError, input::HashedEventRegister, screen};
use crate::{LineNumbers, PagerState};
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::BTreeSet;
use std::{
    convert::{TryFrom, TryInto},
    io::Write,
    time::Duration,
};

use std::collections::hash_map::RandomState;

static INVERT: Lazy<String> = Lazy::new(|| Attribute::Reverse.to_string());
static NORMAL: Lazy<String> = Lazy::new(|| Attribute::NoReverse.to_string());
static ANSI_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new("[\\u001b\\u009b]\\[[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]")
        .unwrap()
});

static WORD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"([\w_]+)|([-?~@#!$%^&*()-+={}\[\]:;\\|'/?<>.,"]+)|\W"#).unwrap());

#[derive(Clone, Copy, Debug, Eq)]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
#[allow(clippy::module_name_repetitions)]
/// Defines modes in which the search can run
pub enum SearchMode {
    /// Find matches from or after the current page
    Forward,
    /// Find matches before the current page
    Reverse,
    /// No search active
    Unknown,
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Unknown
    }
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
    /// Column numbers where each new word start
    pub word_index: Vec<u16>,
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
        let search_char = if ps.search_state.search_mode == SearchMode::Forward {
            '/'
        } else if ps.search_state.search_mode == SearchMode::Reverse {
            '?'
        } else {
            unreachable!();
        };

        let incremental_search_options = IncrementalSearchOpts::from(ps);

        Self {
            ev: None,
            string: String::with_capacity(200),
            input_status: InputStatus::Active,
            cursor_position: 1,
            word_index: Vec::with_capacity(200),
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
        &so.compiled_regex,
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
#[allow(clippy::too_many_lines)]
fn handle_key_press<O, F>(
    out: &mut O,
    so: &mut SearchOpts<'_>,
    incremental_search_condition: F,
) -> crate::Result
where
    O: Write,
    F: Fn(&SearchOpts<'_>) -> bool,
{
    // Bounds between which our cursor can move
    const FIRST_AVAILABLE_COLUMN: u16 = 1;
    let last_available_column: u16 = so.string.len().saturating_add(1).try_into().unwrap();

    // If no event is present, abort
    if so.ev.is_none() {
        return Ok(());
    }

    let populate_word_index = |so: &mut SearchOpts<'_>| {
        so.word_index = WORD
            .find_iter(&so.string)
            .map(|c| c.start().saturating_add(1).try_into().unwrap())
            .collect::<Vec<u16>>();
    };

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
        // If Esc is pressed, cancel the search and also make sure that the search query is
        // ")cleared
        Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.string.clear();
            so.input_status = InputStatus::Cancelled;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            // On backspace, remove the last character just before the cursor from the so.string
            // But if we are at very first character, do nothing.
            if so.cursor_position == FIRST_AVAILABLE_COLUMN {
                return Ok(());
            }
            so.cursor_position = so.cursor_position.saturating_sub(1);
            so.string
                .remove(so.cursor_position.saturating_sub(1).into());
            populate_word_index(so);
            // Update the line
            refresh_display(out, so)?;
            term::move_cursor(out, so.cursor_position, so.rows, false)?;
            out.flush()?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Delete,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            // On delete, remove the character under the cursor from the so.string
            // But if we are at the column right after the last character, do nothing.
            if so.cursor_position >= last_available_column {
                return Ok(());
            }
            so.cursor_position = so.cursor_position.saturating_sub(1);
            so.string
                .remove(<u16 as Into<usize>>::into(so.cursor_position));
            populate_word_index(so);
            so.cursor_position = so.cursor_position.saturating_add(1);
            // Update the line
            refresh_display(out, so)?;
            term::move_cursor(out, so.cursor_position, so.rows, false)?;
            out.flush()?;
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
            if so.cursor_position == FIRST_AVAILABLE_COLUMN {
                return Ok(());
            }
            so.cursor_position = so.cursor_position.saturating_sub(1);
            term::move_cursor(out, so.cursor_position, so.rows, true)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // Find the column number where a word starts which is exactly before the current
            // cursor position
            // If we can't find any such column, jump to the very first available column
            so.cursor_position = *so
                .word_index
                .iter()
                .rfind(|c| c < &&so.cursor_position)
                .unwrap_or(&FIRST_AVAILABLE_COLUMN);
            term::move_cursor(out, so.cursor_position, so.rows, true)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            if so.cursor_position >= last_available_column {
                return Ok(());
            }
            so.cursor_position = so.cursor_position.saturating_add(1);
            term::move_cursor(out, so.cursor_position, so.rows, true)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::CONTROL,
            ..
        }) => {
            // Find the column number where a word starts which is exactly after the current
            // cursor position
            // If we can't find any such column, jump to the very last available column
            so.cursor_position = *so
                .word_index
                .iter()
                .find(|c| c > &&so.cursor_position)
                .unwrap_or(&last_available_column);
            term::move_cursor(out, so.cursor_position, so.rows, true)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Home,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.cursor_position = 1;
            term::move_cursor(out, 1, so.rows, true)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::End,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.cursor_position = so.string.len().saturating_add(1).try_into().unwrap();
            term::move_cursor(out, so.cursor_position, so.rows, true)?;
        }

        Event::Key(event) => {
            // For any character key, without a modifier, insert it into so.string before
            // current cursor position and update the line
            if let KeyCode::Char(c) = event.code {
                so.string
                    .insert(so.cursor_position.saturating_sub(1).into(), c);

                populate_word_index(so);
                refresh_display(out, so)?;
                so.cursor_position = so.cursor_position.saturating_add(1);
                term::move_cursor(out, so.cursor_position, so.rows, false)?;
                out.flush()?;
            }
        }
        _ => return Ok(()),
    }
    Ok(())
}

/// Fetch the search query
///
/// The function will change the prompt to `/` for Forward search or `?` for Reverse search.
/// Next it fetches and handles all events from the terminal screen until [SearchOpts::input_status] isn't
/// set to either [InputStatus::Cancelled] or [InputStatus::Confirmed] by pressing `Esc` or
/// `Enter` respectively.
/// Finally we return
#[cfg(feature = "search")]
pub(crate) fn fetch_input(
    out: &mut impl std::io::Write,
    ps: &PagerState,
) -> Result<FetchInputResult, MinusError> {
    // Set the search character to show at column 0
    let search_char = if ps.search_state.search_mode == SearchMode::Forward {
        '/'
    } else {
        '?'
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
            search::{handle_key_press, InputStatus, SearchOpts},
            SearchMode,
        };
        use crossterm::{
            cursor::MoveTo,
            event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
            terminal::{Clear, ClearType},
        };
        use std::{convert::TryInto, io::Write};

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
                cursor_position: 1,
                word_index: Vec::with_capacity(200),
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

        fn pretest_setup_forward_search() -> (SearchOpts<'static>, Vec<u8>, u16, &'static str) {
            const QUERY_STRING: &str = "this is@complex-text_search?query"; // length = 33
            #[allow(clippy::cast_possible_truncation)]
            let last_movable_column: u16 = (QUERY_STRING.len() as u16) + 1; // 34

            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);

            for c in QUERY_STRING.chars() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Char(c)));
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            }
            assert_eq!(search_opts.cursor_position, last_movable_column);
            (search_opts, out, last_movable_column, QUERY_STRING)
        }

        #[test]
        fn input_sequential_text() {
            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);
            for (i, c) in "text search matches".chars().enumerate() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Char(c)));
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.input_status, InputStatus::Active);
                assert_eq!(search_opts.cursor_position as usize, i + 2);
            }
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Enter));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.word_index, vec![1, 5, 6, 12, 13]);
            assert_eq!(&search_opts.string, "text search matches");
            assert_eq!(search_opts.input_status, InputStatus::Confirmed);
        }

        #[test]
        fn input_complex_sequential_text() {
            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);
            for (i, c) in "this is@complex-text_search?query".chars().enumerate() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Char(c)));
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.input_status, InputStatus::Active);
                assert_eq!(search_opts.cursor_position as usize, i + 2);
            }
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Enter));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.word_index, vec![1, 5, 6, 8, 9, 16, 17, 28, 29]);
            assert_eq!(&search_opts.string, "this is@complex-text_search?query");
            assert_eq!(search_opts.input_status, InputStatus::Confirmed);
        }

        #[test]
        fn home_end_keys() {
            // Setup
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup_forward_search();

            search_opts.ev = Some(make_event_from_keycode(KeyCode::Home));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position as usize, 1);

            search_opts.ev = Some(make_event_from_keycode(KeyCode::End));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position, last_movable_column);
        }

        #[test]
        fn basic_left_arrow_movement() {
            const FIRST_MOVABLE_COLUMN: u16 = 1;
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup_forward_search();
            let query_string_length = last_movable_column - 1;

            // We are currently at the very next column to the last char

            // Check functionality of left arrow key
            // Pressing left arrow moves the cursor towards the beginning of string until it
            // reaches the first char after which pressing it further would not have any effect
            for i in (FIRST_MOVABLE_COLUMN..=query_string_length).rev() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Left));
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.cursor_position, i);
            }
            // Pressing Left arrow any more will not make any effect
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Left));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position, FIRST_MOVABLE_COLUMN);
        }

        #[test]
        fn basic_right_arrow_movement() {
            // Setup
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup_forward_search();
            // Go to the 1st char
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Home));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();

            // Check functionality of right arrow key
            // Pressing right arrow moves the cursor towards the end of string until it
            // reaches the very next column to the last char after which pressing it further would not have any effect
            for i in 2..=last_movable_column {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Right));
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.cursor_position, i);
            }
            // Pressing right arrow any more will not make any effect
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Right));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position, last_movable_column);
        }

        #[test]
        fn right_jump_by_word() {
            const JUMP_COLUMNS: [u16; 10] = [1, 5, 6, 8, 9, 16, 17, 28, 29, LAST_MOVABLE_COLUMN];
            // Setup
            let (mut search_opts, mut out, _last_movable_column, _) =
                pretest_setup_forward_search();
            // LAST_MOVABLE_COLUMN = _last_movable_column = 34
            #[allow(clippy::items_after_statements)]
            const LAST_MOVABLE_COLUMN: u16 = 34;

            // Go to the 1st char
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Home));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();

            let ev = Event::Key(KeyEvent {
                code: KeyCode::Right,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::CONTROL,
                state: KeyEventState::NONE,
            });

            // Jump right word by word
            for i in &JUMP_COLUMNS[1..] {
                search_opts.ev = Some(ev.clone());
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.cursor_position, *i);
            }
            // Pressing ctrl+right will not do anything any keep the cursor at the very next column
            // to the last char
            search_opts.ev = Some(ev);
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position, LAST_MOVABLE_COLUMN);
        }

        #[test]
        fn left_jump_by_word() {
            const JUMP_COLUMNS: [u16; 10] = [1, 5, 6, 8, 9, 16, 17, 28, 29, LAST_MOVABLE_COLUMN];
            // Setup
            let (mut search_opts, mut out, _last_movable_column, _) =
                pretest_setup_forward_search();
            // LAST_MOVABLE_COLUMN = _last_movable_column = 34
            #[allow(clippy::items_after_statements)]
            const LAST_MOVABLE_COLUMN: u16 = 34;

            // We are currently at the very next column to the last char
            let ev = Event::Key(KeyEvent {
                code: KeyCode::Left,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::CONTROL,
                state: KeyEventState::NONE,
            });

            // Jump right word by word
            for i in (JUMP_COLUMNS[..(JUMP_COLUMNS.len() - 1)]).iter().rev() {
                search_opts.ev = Some(ev.clone());
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
                assert_eq!(search_opts.cursor_position, *i);
            }
            // Pressing ctrl+left will not do anything and keep the cursor at the very first column
            search_opts.ev = Some(ev);
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.cursor_position, JUMP_COLUMNS[0]);
        }

        #[test]
        fn esc_key() {
            let (mut search_opts, mut out, _, _) = pretest_setup_forward_search();

            search_opts.ev = Some(make_event_from_keycode(KeyCode::Esc));
            handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            assert_eq!(search_opts.input_status, InputStatus::Cancelled);
        }

        #[test]
        fn forward_sequential_text_input_screen_data() {
            let (search_opts, out, _last_movable_column, query_string) =
                pretest_setup_forward_search();

            let mut result_out = Vec::with_capacity(1500);

            // Try to recreate the behaviour of handle_key_press when new char is entered
            let mut string = String::with_capacity(query_string.len());
            let mut cursor_position: u16 = 1;
            for c in query_string.chars() {
                string.push(c);
                cursor_position = cursor_position.saturating_add(1);
                write!(
                    result_out,
                    "{move_to_prompt}\r{clear_line}/{string}{move_to_position}",
                    move_to_prompt = MoveTo(0, search_opts.rows),
                    clear_line = Clear(ClearType::CurrentLine),
                    move_to_position = MoveTo(cursor_position, search_opts.rows),
                )
                .unwrap();
            }
            assert_eq!(out, result_out);
        }

        #[test]
        fn backward_sequential_text_input_screen_data() {
            const QUERY_STRING: &str = "this is@complex-text_search?query"; // length = 33
            #[allow(clippy::cast_possible_truncation)]
            const LAST_MOVABLE_COLUMN: u16 = (QUERY_STRING.len() as u16) + 1; // 34

            let mut search_opts = new_search_opts(SearchMode::Reverse);
            let mut out = Vec::with_capacity(1500);

            for c in QUERY_STRING.chars() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Char(c)));
                handle_key_press(&mut out, &mut search_opts, |_| false).unwrap();
            }
            assert_eq!(search_opts.cursor_position, LAST_MOVABLE_COLUMN);

            let mut result_out = Vec::with_capacity(1500);

            // Try to recreate the behaviour of handle_key_press when new char is entered
            let mut string = String::with_capacity(QUERY_STRING.len());
            let mut cursor_position: u16 = 1;
            for c in QUERY_STRING.chars() {
                string.push(c);
                cursor_position = cursor_position.saturating_add(1);
                write!(
                    result_out,
                    "{move_to_prompt}\r{clear_line}?{string}{move_to_position}",
                    move_to_prompt = MoveTo(0, search_opts.rows),
                    clear_line = Clear(ClearType::CurrentLine),
                    move_to_position = MoveTo(cursor_position, search_opts.rows),
                )
                .unwrap();
            }
            assert_eq!(out, result_out);
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

    #[allow(clippy::trivial_regex)]
    mod highlighting {
        use std::collections::BTreeSet;

        use crate::search::{highlight_line_matches, next_nth_match, INVERT, NORMAL};
        use crate::PagerState;
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
