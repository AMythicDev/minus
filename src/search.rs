#![allow(unused_imports)]
use crate::error::AlternateScreenPagingError;
#[cfg(feature = "search")]
use crate::utils::SearchMode;
use crate::Pager;
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use std::{convert::TryFrom, time::Duration};

/// Fetch the search query asynchronously
#[cfg(all(feature = "static_output", feature = "search"))]
pub(crate) fn fetch_input_blocking(
    out: &mut impl std::io::Write,
    search_mode: crate::utils::SearchMode,
    rows: usize,
) -> Result<String, AlternateScreenPagingError> {
    // Place the cursor at the beginning of very last line of the terminal and clear
    // the prompt and show the cursor
    #[allow(clippy::cast_possible_truncation)]
    write!(
        out,
        "{}{}{}{}",
        MoveTo(0, rows as u16),
        Clear(ClearType::CurrentLine),
        if search_mode == SearchMode::Forward {
            "/"
        } else {
            "?"
        },
        cursor::Show
    )?;
    out.flush()?;
    let mut string = String::new();
    loop {
        if event::poll(Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            match event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))? {
                // If Esc is pressed, cancel the search
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    return Ok(String::new());
                }
                // On backspace, pop the last character from the string
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    string.pop();
                    // Update the line
                    write!(out, "\r{}/{}", Clear(ClearType::CurrentLine), string)?;
                    out.flush()?;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    // Return the string when enter is pressed
                    return Ok(string);
                }
                Event::Key(event) => {
                    // For any character key, without a modifier, append it to the
                    // string and update the line
                    if let KeyCode::Char(c) = event.code {
                        string.push(c);
                        write!(out, "\r/{}", string)?;
                        out.flush()?;
                    }
                }
                _ => continue,
            }
        }
    }
}
/// Fetch input anychronously
// This is similar to fetch_input_blocking except that it is async
#[cfg(all(
    any(feature = "async_std_lib", feature = "tokio_lib"),
    feature = "search"
))]
pub(crate) async fn fetch_input(
    out: &mut impl std::io::Write,
    search_mode: crate::utils::SearchMode,
    rows: usize,
) -> Result<String, AlternateScreenPagingError> {
    #[allow(clippy::cast_possible_truncation)]
    write!(
        out,
        "{}{}{}{}",
        MoveTo(0, rows as u16),
        Clear(ClearType::CurrentLine),
        if search_mode == SearchMode::Forward {
            "/"
        } else {
            "?"
        },
        cursor::Show
    )?;
    out.flush()?;
    let mut string = String::new();
    loop {
        if event::poll(Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            match event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))? {
                Event::Key(KeyEvent {
                    code: KeyCode::Esc,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    return Ok(String::new());
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Backspace,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    string.pop();
                    write!(out, "\r{}/{}", Clear(ClearType::CurrentLine), string)?;
                    out.flush()?;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    return Ok(string);
                }
                Event::Key(event) => {
                    if let KeyCode::Char(c) = event.code {
                        string.push(c);
                        write!(out, "\r/{}", string)?;
                        out.flush()?;
                    }
                }
                _ => continue,
            }
        }
    }
}

/// Highlight all matches of the given query and return the coordinate of each match
#[cfg(feature = "search")]
pub(crate) fn highlight_search(pager: &mut Pager) -> Result<(), regex::Error> {
    let pattern = regex::Regex::new(&pager.search_term)?;
    let mut coordinates: Vec<u16> = Vec::new();
    pager.search_lines = pager.lines.clone();

    let mut idx = 0;

    for line in pager.search_lines.iter_mut() {
        for term_line in line.iter_mut() {
            if let Some(cap) = pattern.captures(&(*term_line).to_string()) {
                let text = format!("{}{}{}", Attribute::Reverse, &cap[0], Attribute::Reset);
                let text = text.as_str();
                let replace = pattern.replace_all(&term_line, text).to_string();
                coordinates.push(u16::try_from(idx).unwrap());

                *term_line = replace;
            }
            idx += 1;
        }
    }
    pager.search_idx = coordinates;
    Ok(())
}

pub(crate) fn highlight_line_matches(
    line: &mut String,
    query: &str,
) -> Result<(), regex::Error> {
    let pattern = regex::Regex::new(query)?;

    if let Some(cap) = pattern.captures(line) {
        let text = format!("{}{}{}", Attribute::Reverse, &cap[0], Attribute::Reset);
        let text = text.as_str();
        *line = pattern.replace_all(&line, text).to_string();
    }

    Ok(())
}
