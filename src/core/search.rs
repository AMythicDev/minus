//! Provides functions related to searching

#![allow(unused_imports)]
use crate::error::MinusError;
use crate::PagerState;
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::{
    convert::{TryFrom, TryInto},
    io::Write,
    time::Duration,
};

use super::utils::term;

static INVERT: Lazy<String> = Lazy::new(|| Attribute::Reverse.to_string());
static NORMAL: Lazy<String> = Lazy::new(|| Attribute::NoReverse.to_string());
static ANSI_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new("[\\u001b\\u009b]\\[[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]")
        .unwrap()
});

// TODO deal with "
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

struct SearchOpts {
    ev: Option<Event>,
    string: String,
    is_input_done: bool,
    cursor_position: u16,
    word_index: Vec<usize>,
    search_char: char,
    rows: u16,
}

fn handle_key_press<O>(out: &mut O, so: &mut SearchOpts) -> crate::Result
where
    O: Write,
{
    if so.ev.is_none() {
        return Ok(());
    }
    let populate_word_index = |so: &mut SearchOpts| {
        so.word_index = WORD
            .find_iter(&so.string)
            .map(|c| c.start().saturating_add(1))
            .collect::<Vec<usize>>();
    };
    let update_input_display = |out: &mut O, so: &mut SearchOpts| -> crate::Result {
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
        // If Esc is pressed, cancel the search
        Event::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            so.is_input_done = true;
        }
        // On backspace, pop the last character from the so.string
        Event::Key(KeyEvent {
            code: KeyCode::Backspace,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            if so.cursor_position == 1 {
                return Ok(());
            }
            so.cursor_position = so.cursor_position.saturating_sub(1);
            so.string
                .remove(so.cursor_position.saturating_sub(1).into());
            populate_word_index(so);
            // Update the line
            update_input_display(out, so)?;
            term::move_cursor(out, so.cursor_position, so.rows, false)?;
            out.flush()?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Delete,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            if so.cursor_position == 1
                || <u16 as Into<usize>>::into(so.cursor_position) > so.string.len()
            {
                return Ok(());
            }
            so.cursor_position = so.cursor_position.saturating_sub(1);
            so.string
                .remove(<u16 as Into<usize>>::into(so.cursor_position));
            populate_word_index(so);
            so.cursor_position = so.cursor_position.saturating_add(1);
            // Update the line
            update_input_display(out, so)?;
            term::move_cursor(out, so.cursor_position, so.rows, false)?;
            out.flush()?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            write!(out, "{}", cursor::Hide)?;
            // Return the so.string when enter is pressed
            so.is_input_done = true;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Left,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            if so.cursor_position == 1 {
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
            so.cursor_position = u16::try_from(
                *so.word_index
                    .iter()
                    .rfind(|c| c < &&(so.cursor_position as usize))
                    .unwrap_or(&(so.cursor_position as usize)),
            )
            .unwrap();
            term::move_cursor(out, so.cursor_position, so.rows, true)?;
        }
        Event::Key(KeyEvent {
            code: KeyCode::Right,
            modifiers: KeyModifiers::NONE,
            ..
        }) => {
            if <u16 as Into<usize>>::into(so.cursor_position) > so.string.len() {
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
            so.cursor_position = u16::try_from(
                *so.word_index
                    .iter()
                    .find(|c| c > &&(so.cursor_position as usize))
                    .unwrap_or(&(so.cursor_position as usize))
            )
            .unwrap();
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
            // For any character key, without a modifier, append it to the
            // so.string and update the line
            if let KeyCode::Char(c) = event.code {
                so.string
                    .insert(so.cursor_position.saturating_sub(1).into(), c);

                populate_word_index(so);
                update_input_display(out, so)?;
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
pub fn fetch_input(
    out: &mut impl std::io::Write,
    search_mode: SearchMode,
    rows: usize,
) -> Result<String, MinusError> {
    // Set the search character to show at column 0
    let search_char = if search_mode == SearchMode::Forward {
        '/'
    } else {
        '?'
    };

    // Initial setup
    // - Place the cursor at the beginning of prompt line
    // - Clear the prompt
    // - Write the search character and
    // - Show the cursor
    #[allow(clippy::cast_possible_truncation)]
    write!(
        out,
        "{}{}{}{}",
        MoveTo(0, rows as u16),
        Clear(ClearType::CurrentLine),
        search_char,
        cursor::Show
    )?;
    out.flush()?;

    let mut search_opts = SearchOpts {
        search_char,
        is_input_done: false,
        ev: None,
        word_index: Vec::with_capacity(200),
        cursor_position: 1,
        string: String::new(),
        rows: rows.try_into().unwrap(),
    };

    loop {
        if event::poll(Duration::from_millis(100)).map_err(|e| MinusError::HandleEvent(e.into()))? {
            let ev = event::read().map_err(|e| MinusError::HandleEvent(e.into()))?;
            search_opts.ev = Some(ev);
            handle_key_press(out, &mut search_opts)?;
            search_opts.ev = None;
        }
        if search_opts.is_input_done {
            break;
        }
    }
    // Teardown: almost opposite of setup
    write!(
        out,
        "{}{}{}",
        MoveTo(0, rows as u16),
        Clear(ClearType::CurrentLine),
        cursor::Hide
    )?;
    out.flush()?;
    Ok(search_opts.string)
}

/// Highlights the search match
///
/// The first return value returns the line that has all the search matches highlighted
/// The second tells whether a search match was actually found
pub fn highlight_line_matches(line: &str, query: &regex::Regex) -> (String, bool) {
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
pub fn next_nth_match(ps: &mut PagerState, n: usize) {
    // Find the first match that's after the upper_mark, then set the mark to that match.
    // If we can't find one, just set it to the last match
    if let Some(nearest_idx) = ps.search_idx.iter().position(|i| *i > ps.upper_mark) {
        ps.search_mark = nearest_idx.saturating_add(n).saturating_sub(1);

        if ps.search_mark > ps.search_idx.len().saturating_sub(1) {
            ps.search_mark = ps.search_idx.len().saturating_sub(1);
        }
    } else {
        ps.search_mark = ps.search_idx.len().saturating_sub(1);
    }

    // And set the upper_mark to that match so that we scroll to it
    if let Some(idx) = ps.search_idx.iter().nth(ps.search_mark) {
        ps.upper_mark = *idx;
    }
}

#[cfg(test)]
mod tests {
    mod input_handling {
        use crate::{
            minus_core::search::{handle_key_press, SearchOpts},
            SearchMode,
        };
        use crossterm::{
            cursor::MoveTo,
            event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
            terminal::{Clear, ClearType},
        };
        use std::io::Write;

        const fn new_search_opts(sm: SearchMode) -> SearchOpts {
            let search_char = match sm {
                SearchMode::Forward => '/',
                SearchMode::Reverse => '?',
                _ => unreachable!(),
            };
            SearchOpts {
                search_char,
                is_input_done: false,
                ev: None,
                word_index: Vec::new(),
                cursor_position: 1,
                string: String::new(),
                rows: 80,
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

        fn pretest_setup() -> (SearchOpts, Vec<u8>, u16, &'static str) {
            const QUERY_STRING: &str = "this is@complex-text_search?query"; // length = 33
            const LAST_MOVABLE_COLUMN: u16 = (QUERY_STRING.len() as u16) + 1; // 34

            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);

            for c in QUERY_STRING.chars() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Char(c)));
                handle_key_press(&mut out, &mut search_opts).unwrap();
            }
            assert_eq!(search_opts.cursor_position, LAST_MOVABLE_COLUMN);
            (search_opts, out, LAST_MOVABLE_COLUMN, QUERY_STRING)
        }

        #[test]
        fn input_sequential_text() {
            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);
            for (i, c) in "text search matches".chars().enumerate() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Char(c)));
                handle_key_press(&mut out, &mut search_opts).unwrap();
                assert!(!search_opts.is_input_done);
                assert_eq!(search_opts.cursor_position as usize, i + 2);
            }
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Enter));
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.word_index, vec![1, 5, 6, 12, 13]);
            assert_eq!(&search_opts.string, "text search matches");
            assert!(search_opts.is_input_done);
        }

        #[test]
        fn input_complex_sequential_text() {
            let mut search_opts = new_search_opts(SearchMode::Forward);
            let mut out = Vec::with_capacity(1500);
            for (i, c) in "this is@complex-text_search?query".chars().enumerate() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Char(c)));
                handle_key_press(&mut out, &mut search_opts).unwrap();
                assert!(!search_opts.is_input_done);
                assert_eq!(search_opts.cursor_position as usize, i + 2);
            }
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Enter));
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.word_index, vec![1, 5, 6, 8, 9, 16, 17, 28, 29]);
            assert_eq!(&search_opts.string, "this is@complex-text_search?query");
            assert!(search_opts.is_input_done);
        }

        #[test]
        fn home_end_keys() {
            // Setup
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup();

            search_opts.ev = Some(make_event_from_keycode(KeyCode::Home));
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.cursor_position as usize, 1);

            search_opts.ev = Some(make_event_from_keycode(KeyCode::End));
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.cursor_position, last_movable_column);
        }

        #[test]
        fn basic_left_arrow_movement() {
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup();
            const FIRST_MOVABLE_COLUMN: u16 = 1;
            let query_string_length = last_movable_column - 1;

            // We are currently at the very next column to the last char

            // Check functionality of left arrow key
            // Pressing left arrow moves the cursor towards the beginning of string until it
            // reaches the first char after which pressing it furthur would not have any effect
            for i in (FIRST_MOVABLE_COLUMN..=query_string_length).rev() {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Left));
                handle_key_press(&mut out, &mut search_opts).unwrap();
                assert_eq!(search_opts.cursor_position, i);
            }
            // Pressing Left arrow any more will not make any effect
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Left));
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.cursor_position, FIRST_MOVABLE_COLUMN);
        }

        #[test]
        fn basic_right_arrow_movement() {
            // Setup
            let (mut search_opts, mut out, last_movable_column, _) = pretest_setup();
            // Go to the 1st char
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Home));
            handle_key_press(&mut out, &mut search_opts).unwrap();

            // Check functionality of right arrow key
            // Pressing right arrow moves the cursor towards the end of string until it
            // reaches the very next column to the last char after which pressing it furthur would not have any effect
            for i in 2..=last_movable_column {
                search_opts.ev = Some(make_event_from_keycode(KeyCode::Right));
                handle_key_press(&mut out, &mut search_opts).unwrap();
                assert_eq!(search_opts.cursor_position, i);
            }
            // Pressing right arrow any more will not make any effect
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Right));
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.cursor_position, last_movable_column);
        }

        #[test]
        fn right_jump_by_word() {
            // Setup
            let (mut search_opts, mut out, _last_movable_column, _) = pretest_setup();
            // LAST_MOVABLE_COLUMN = _last_movable_column = 34
            const LAST_MOVABLE_COLUMN: u16 = 34;
            const JUMP_COLUMNS: [u16; 10] = [1, 5, 6, 8, 9, 16, 17, 28, 29, LAST_MOVABLE_COLUMN];

            // Go to the 1st char
            search_opts.ev = Some(make_event_from_keycode(KeyCode::Home));
            handle_key_press(&mut out, &mut search_opts).unwrap();

            let ev = Event::Key(KeyEvent {
                code: KeyCode::Right,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::CONTROL,
                state: KeyEventState::NONE,
            });

            // Jump right word by word
            for i in &JUMP_COLUMNS[1..] {
                search_opts.ev = Some(ev.clone());
                handle_key_press(&mut out, &mut search_opts).unwrap();
                assert_eq!(search_opts.cursor_position, *i)
            }
            // Pressing ctrl+right will not do anything any keep the cursor at the very next column
            // to the last char
            search_opts.ev = Some(ev.clone());
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.cursor_position, LAST_MOVABLE_COLUMN);
        }

        #[test]
        fn left_jump_by_word() {
            // Setup
            let (mut search_opts, mut out, _last_movable_column, _) = pretest_setup();
            // LAST_MOVABLE_COLUMN = _last_movable_column = 34
            const LAST_MOVABLE_COLUMN: u16 = 34;
            const JUMP_COLUMNS: [u16; 10] = [1, 5, 6, 8, 9, 16, 17, 28, 29, LAST_MOVABLE_COLUMN];

            // We are currently at the very next column to the last char
            let ev = Event::Key(KeyEvent {
                code: KeyCode::Left,
                kind: KeyEventKind::Press,
                modifiers: KeyModifiers::CONTROL,
                state: KeyEventState::NONE,
            });

            // Jump right word by word
            for i in (&JUMP_COLUMNS[..(JUMP_COLUMNS.len() - 1)]).iter().rev() {
                search_opts.ev = Some(ev.clone());
                handle_key_press(&mut out, &mut search_opts).unwrap();
                assert_eq!(search_opts.cursor_position, *i)
            }
            // Pressing ctrl+left will not do anything and keep the cursor at the very first column
            search_opts.ev = Some(ev.clone());
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert_eq!(search_opts.cursor_position, JUMP_COLUMNS[0]);
        }

        #[test]
        fn esc_key() {
            let (mut search_opts, mut out, _, _) = pretest_setup();

            search_opts.ev = Some(make_event_from_keycode(KeyCode::Esc));
            handle_key_press(&mut out, &mut search_opts).unwrap();
            assert!(search_opts.is_input_done);
        }

        #[test]
        fn sequential_text_input_screen_data() {
            let (search_opts, out, _last_movable_column, query_string) = pretest_setup();

            let mut result_out = Vec::with_capacity(1500);

            // Try to recreate the behaviour of handle_key_press when new char is entered
            let mut string = String::with_capacity(query_string.len());
            let mut cursor_position: u16 = 1;
            for c in query_string.chars() {
                string.push(c);
                cursor_position = cursor_position.saturating_add(1);
                write!(
                    result_out,
                    "\r{clear_line}{search_char}{string}{move_to_position}",
                    clear_line = Clear(ClearType::CurrentLine),
                    search_char = '/',
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
            let mut pager = PagerState::new().unwrap();
            pager.search_mark = 0;
            // A sample index for mocking actual search index matches
            pager.search_idx = BTreeSet::from([2, 10, 15, 17, 50]);
            for i in &pager.search_idx.clone() {
                next_nth_match(&mut pager, 1);
                assert_eq!(pager.upper_mark, *i);
                pager.search_mark += 1;
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
