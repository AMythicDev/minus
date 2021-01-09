use crate::error::AlternateScreenPagingError;
use crate::Pager;
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use std::time::Duration;

/// Fetch the search query asynchronously
#[cfg(feature = "static_output")]
pub(crate) fn fetch_input_blocking(
    out: &mut impl std::io::Write,
    rows: usize,
) -> Result<String, AlternateScreenPagingError> {
    // Place the cursor at the beginning of very last line of the terminal and clear
    // the prompt and show the cursor
    #[allow(clippy::cast_possible_truncation)]
    write!(
        out,
        "{}{}/{}",
        Clear(ClearType::CurrentLine),
        MoveTo(0, rows as u16),
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
#[cfg(any(feature = "async_std_lib", feature = "tokio_lib"))]
pub(crate) async fn fetch_input(
    out: &mut impl std::io::Write,
    rows: usize,
) -> Result<String, AlternateScreenPagingError> {
    #[allow(clippy::cast_possible_truncation)]
    write!(
        out,
        "{}{}/{}",
        Clear(ClearType::CurrentLine),
        MoveTo(0, rows as u16),
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
pub(crate) fn highlight_search(
    pager: &mut Pager,
    query: &str,
) -> Result<Vec<(u16, u16)>, regex::Error> {
    let pattern = regex::Regex::new(query)?;
    let mut coordinates: Vec<(u16, u16)> = Vec::new();
    let mut lines: Vec<String> = pager
        .lines
        .lines()
        .map(std::string::ToString::to_string)
        .collect();

    for (i, line) in lines.iter_mut().enumerate() {
        if let Some(cap) = pattern.captures(&line.clone()) {
            let text = format!("{}{}{}", Attribute::Reverse, &cap[0], Attribute::Reset);
            let text = text.as_str();
            let replace = pattern.replace_all(line, text).to_string();

            find(line.clone(), &cap[0]).iter().for_each(|x| {
                #[allow(clippy::cast_possible_truncation)]
                coordinates.push((*x as u16, i as u16));
            });

            *line = replace;
        }
    }
    pager.lines = lines.join("\n");
    pager.lines.push('\n');
    Ok(coordinates)
}

pub(crate) fn find(mut text: String, query: &str) -> Vec<usize> {
    // Initialize a vector of points
    let mut points: Vec<usize> = Vec::new();
    // Mark of searching in the line. This tells upto what poistion the search is done
    let mut searched = 0;
    // Replace all tabs with 6 spaces. There is a probably better way to do this
    text = text.replace('\t', "      ");

    while let Some(x) = text.find(&query) {
        // Push the point of the first character of the term
        points.push(searched + x);
        // Calculate the length of the text including the entire query
        let truncate = x + query.char_indices().count();
        // Drain everything upto the point
        text.drain(..truncate);
        // Update the searched
        searched += truncate;
    }
    points
}
