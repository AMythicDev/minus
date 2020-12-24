use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use std::time::Duration;
use crate::error::AlternateScreenPagingError;
use crate::Pager;

pub(crate) async fn fetch_input(
    out: &mut impl std::io::Write,
    rows: usize
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
) -> Result<(), regex::Error> {
    let pattern = regex::Regex::new(query)?;
    for cap in pattern.captures(&pager.lines.clone()) {
        let text = &format!(
            "{}{}{}",
            Attribute::Reverse,
            &cap[0],
            Attribute::Reset
        );
        pager.lines = pager.lines.replace(&cap[0], text);
    }
    Ok(())
}

pub(crate) fn locate_match(p: &mut Pager, query: &str, rev: bool) {
    let mut lines = p.lines.lines().collect::<Vec<&str>>();
    let line_count = p.lines.lines().count();

    // TODO: Improve this section using .take and .skip
    /// More improvements could possibly be done

    if rev {
        lines.reverse();
        for (i, line) in (&lines[line_count - p.upper_mark..line_count]).iter().enumerate() {
            if line.contains(query) {
                p.upper_mark = p.upper_mark - i -1;
                break;
            }
        }
    } else {
        for (i, line) in (&lines[p.upper_mark..line_count]).iter().enumerate() {
            if line.contains(query) && p.upper_mark != i {
                p.upper_mark = i;
                break;
            }
        }
    }
}