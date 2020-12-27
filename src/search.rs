use crate::error::AlternateScreenPagingError;
use crate::Pager;
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use std::time::Duration;

pub(crate) fn fetch_input_blocking(
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
            *line = replace;

            let x = line.find(text).unwrap();
            #[allow(clippy::cast_possible_truncation)]
            coordinates.push((x as u16, i.saturating_sub(1) as u16));
        }
    }
    pager.lines = lines.join("\n");
    pager.lines.push('\n');
    Ok(coordinates)
}
