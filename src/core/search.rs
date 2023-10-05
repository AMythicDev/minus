//! Provides functions related to searching

#![allow(unused_imports)]
use super::utils::{display, term, text};
use crate::{error::MinusError, input::HashedEventRegister};
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
    incremental_search_result: Option<IncrementalSearchResult>,
    compiled_regex: Option<Regex>,
}

/// Options to control incremental search
///
/// The values of the fields should not be modified at any point otherwise it may lead to
/// unexpected text display while doing a incremental search. One exception to this is 
/// `upper_mark` which is modified by the priavte `handle_key_press` function.
pub struct IncrementalSearchOpts<'a> {
    /// Current upper mark
    pub upper_mark: usize,
    /// Text to be searched
    pub text: &'a String,
    /// Current status of line numbering
    pub line_numbers: LineNumbers,
    /// Reference tp [PagerState::formatted_lines] before starting of search prompt
    pub initial_formatted_lines: &'a Vec<String>,
    /// Value of [PagerState::upper_mark] before starting of search prompt
    pub initial_upper_mark: usize,
}

impl<'a> From<&'a PagerState> for IncrementalSearchOpts<'a> {
    fn from(ps: &'a PagerState) -> Self {
        Self {
            text: &ps.lines,
            line_numbers: ps.line_numbers,
            initial_upper_mark: ps.upper_mark,
            upper_mark: ps.upper_mark,
            initial_formatted_lines: &ps.formatted_lines,
        }
    }
}

#[allow(clippy::fallible_impl_from)]
impl<'a> From<&'a PagerState> for SearchOpts<'a> {
    fn from(ps: &'a PagerState) -> Self {
        let search_char = if ps.search_mode == SearchMode::Forward {
            '/'
        } else if ps.search_mode == SearchMode::Reverse {
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
            incremental_search_result: None,
            compiled_regex: None,
            search_mode: ps.search_mode,
        }
    }
}

/// Status of the search prompt
#[derive(Debug, PartialEq, Clone)]
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
    pub const fn done(&self) -> bool {
        matches!(self, Self::Cancelled | Self::Confirmed)
    }
}

pub(crate) struct FetchInputResult {
    pub(crate) string: String,
    pub(crate) incremental_search_result: Option<IncrementalSearchResult>,
    pub(crate) compiled_regex: Option<Regex>,
}

impl FetchInputResult {
    const fn new_empty() -> Self {
        Self {
            string: String::new(),
            incremental_search_result: None,
            compiled_regex: None,
        }
    }
}

pub(crate) struct IncrementalSearchResult {
    pub(crate) formatted_lines: Vec<String>,
    pub(crate) searh_mark: usize,
    pub(crate) search_idx: BTreeSet<usize>,
    pub(crate) upper_mark: usize,
}

fn run_incremental_search<'a, F, O>(
    out: &mut O,
    so: &'a SearchOpts<'a>,
    incremental_search_condition: F,
) -> crate::Result<Option<IncrementalSearchResult>>
where
    O: Write,
    F: Fn(&'a SearchOpts) -> bool,
{
    if so.incremental_search_options.is_none() {
        return Ok(None);
    }
    let incremental_search_options = so.incremental_search_options.as_ref().unwrap();
    let mut initial_upper_mark = incremental_search_options.initial_upper_mark;

    let should_proceed = incremental_search_condition(so);

    if so.incremental_search_result.is_some() && (so.compiled_regex.is_none() || !(should_proceed))
    {
        display::write_text_checked(
            out,
            incremental_search_options.initial_formatted_lines,
            so.rows.into(),
            &mut initial_upper_mark,
        )?;
        return Ok(None);
    }

    if so.compiled_regex.is_none() && !should_proceed {
        return Ok(None);
    }

    let format_result = text::make_format_lines(
        incremental_search_options.text,
        incremental_search_options.line_numbers,
        so.cols.try_into().unwrap(),
        &so.compiled_regex,
    );

    let position_of_next_match = next_nth_match(
        &format_result.append_search_idx,
        incremental_search_options.upper_mark,
        1,
    );

    let mut upper_mark;
    if let Some(idx) = format_result
        .append_search_idx
        .iter()
        .nth(position_of_next_match)
    {
        upper_mark = *idx;
    } else {
        display::write_text_checked(
            out,
            incremental_search_options.initial_formatted_lines,
            so.rows.into(),
            &mut initial_upper_mark,
        )?;
        return Ok(None);
    }
    display::write_text_checked(out, &format_result.lines, so.rows.into(), &mut upper_mark)?;
    Ok(Some(IncrementalSearchResult {
        formatted_lines: format_result.lines,
        searh_mark: position_of_next_match,
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
        so.compiled_regex = Regex::new(&so.string).ok();

        so.incremental_search_result =
            run_incremental_search(out, so, incremental_search_condition)?;
        if let Some(IncrementalSearchResult { upper_mark, .. }) = so.incremental_search_result {
            so.incremental_search_options.as_mut().unwrap().upper_mark = upper_mark;
        }

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
            // current cursor positon and update the line
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
/// The function will change the prompt to `/` for Forward search or `?` for Reverse search
/// It will then store the query in a String and return it when `Return` key is pressed
/// or return with a empty string if so match is found.
#[cfg(feature = "search")]
pub(crate) fn fetch_input(
    out: &mut impl std::io::Write,
    ps: &PagerState,
) -> Result<FetchInputResult, MinusError> {
    // Set the search character to show at column 0
    let search_char = if ps.search_mode == SearchMode::Forward {
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

    let text_lines_count = ps.lines.lines().count();

    let mut search_opts = SearchOpts::from(ps);

    loop {
        if event::poll(Duration::from_millis(100)).map_err(|e| MinusError::HandleEvent(e.into()))? {
            let ev = event::read().map_err(|e| MinusError::HandleEvent(e.into()))?;
            search_opts.ev = Some(ev);
            handle_key_press(
                out,
                &mut search_opts,
                |search_opts: &SearchOpts<'_>| -> bool {
                    search_opts.string.len() >= 2 && text_lines_count < 5000
                },
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
        InputStatus::Confirmed => {
            FetchInputResult {
                string: search_opts.string,
                // TODO: Allow incremental search result to be propagated upward
                incremental_search_result: search_opts.incremental_search_result,
                compiled_regex: search_opts.compiled_regex,
            }
        }
    };
    Ok(fetch_input_result)
}

/// Highlights the search match
///
/// The first return value returns the line that has all the search matches highlighted
/// The second tells whether a search match was actually found
pub(crate) fn highlight_line_matches(line: &str, query: &regex::Regex) -> (String, bool) {
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
        // Find how many invert|normal markers appear before this escape
        let match_count = matches.iter().take_while(|m| **m <= esc.0).count();

        if match_count % 2 == 1 {
            // if == 1, then it's either at the same spot as the start of an invert, or in the
            // middle of an invert. Either way we don't want to place it in.
            continue;
        }

        // find the number of invert strings and number of uninvert strings that have been
        // inserted up to this point in the string
        let num_invert = match_count / 2;
        let num_normal = match_count - num_invert;

        // calculate the index which this escape should be re-inserted at by adding
        // its position in the stripped string to the total length of the ansi escapes
        // (both highlighting and the ones from the original string).
        let pos =
            esc.0 + inserted_escs_len + (num_invert * INVERT.len()) + (num_normal * NORMAL.len());

        // insert the escape back in
        inverted.insert_str(pos, esc.1);

        // increment the length of the escapes inserted back in
        inserted_escs_len += esc.1.len();
    }

    (inverted, true)
}

/// Set [`PagerState::search_mark`] to move to the next match
///
/// This function will continue looping untill it finds a match that is after the
/// [`PagerState::upper_mark`]
#[must_use]
pub(crate) fn next_nth_match(search_idx: &BTreeSet<usize>, upper_mark: usize, n: usize) -> usize {
    // Find the index of the match that's exactly after the upper_mark.
    // One we find that, we add n-1 to it to get the next nth match after upper_mark
    let mut position_of_next_match;
    if let Some(nearest_idx) = search_idx.iter().position(|i| *i > upper_mark) {
        position_of_next_match = nearest_idx.saturating_add(n).saturating_sub(1);

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

    position_of_next_match
    // And set the upper_mark to that match so that we scroll to it
}

#[cfg(test)]
mod tests {
    mod input_handling {
        use crate::{
            minus_core::search::{handle_key_press, InputStatus, SearchOpts},
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
                incremental_search_result: None,
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
            // reaches the first char after which pressing it furthur would not have any effect
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
            // reaches the very next column to the last char after which pressing it furthur would not have any effect
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

    #[allow(clippy::trivial_regex)]
    mod highlighting {
        use std::collections::BTreeSet;

        use crate::minus_core::search::{highlight_line_matches, next_nth_match, INVERT, NORMAL};
        use crate::PagerState;
        use crossterm::style::Attribute;
        use regex::Regex;

        // generic escape code
        const ESC: &str = "\x1b[34m";
        const NONE: &str = "\x1b[0m";

        #[test]
        fn test_next_match() {
            // A sample index for mocking actual search index matches
            let search_idx = BTreeSet::from([2, 10, 15, 17, 50]);
            let mut upper_mark = 0;
            let mut search_mark;
            for (i, v) in search_idx.iter().enumerate() {
                search_mark = next_nth_match(&search_idx, upper_mark, 1);
                assert_eq!(search_mark, i);
                dbg!(search_mark);
                let next_upper_mark = *search_idx.iter().nth(search_mark).unwrap();
                assert_eq!(next_upper_mark, *v);
                upper_mark = next_upper_mark;
            }
        }

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

            assert_eq!(highlight_line_matches(&line, &pat).0, result);
        }

        #[test]
        fn no_match() {
            let orig = "no match";
            let res = highlight_line_matches(orig, &Regex::new("test").unwrap());
            assert_eq!(res.0, orig.to_string());
        }

        #[test]
        fn single_match_no_esc() {
            let res = highlight_line_matches("this is a test", &Regex::new(" a ").unwrap());
            assert_eq!(res.0, format!("this is{} a {}test", *INVERT, *NORMAL));
        }

        #[test]
        fn multi_match_no_esc() {
            let res = highlight_line_matches("test another test", &Regex::new("test").unwrap());
            assert_eq!(
                res.0,
                format!("{i}test{n} another {i}test{n}", i = *INVERT, n = *NORMAL)
            );
        }

        #[test]
        fn esc_outside_match() {
            let res = highlight_line_matches(
                &format!("{ESC}color{NONE} and test"),
                &Regex::new("test").unwrap(),
            );
            assert_eq!(
                res.0,
                format!("{}color{} and {}test{}", ESC, NONE, *INVERT, *NORMAL)
            );
        }

        #[test]
        fn esc_end_in_match() {
            let orig = format!("this {ESC}is a te{NONE}st");
            let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
            assert_eq!(
                res.0,
                format!("this {}is a {}test{}", ESC, *INVERT, *NORMAL)
            );
        }

        #[test]
        fn esc_start_in_match() {
            let orig = format!("this is a te{ESC}st again{NONE}");
            let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
            assert_eq!(
                res.0,
                format!("this is a {}test{} again{}", *INVERT, *NORMAL, NONE)
            );
        }

        #[test]
        fn esc_around_match() {
            let orig = format!("this is {ESC}a test again{NONE}");
            let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
            assert_eq!(
                res.0,
                format!("this is {}a {}test{} again{}", ESC, *INVERT, *NORMAL, NONE)
            );
        }

        #[test]
        fn esc_within_match() {
            let orig = format!("this is a t{ESC}es{NONE}t again");
            let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
            assert_eq!(res.0, format!("this is a {}test{} again", *INVERT, *NORMAL));
        }

        #[test]
        fn multi_escape_match() {
            let orig = format!("this {ESC}is a te{NONE}st again {ESC}yeah{NONE} test",);
            let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
            assert_eq!(
                res.0,
                format!(
                    "this {e}is a {i}test{n} again {e}yeah{nn} {i}test{n}",
                    e = ESC,
                    i = *INVERT,
                    n = *NORMAL,
                    nn = NONE
                )
            );
        }
    }
}
