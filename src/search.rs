#![allow(unused_imports)]
use crate::error::AlternateScreenPagingError;
use crate::Pager;
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{Clear, ClearType},
};
use std::{convert::TryFrom, time::Duration};

#[derive(PartialEq, Clone, Copy, Debug)]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
#[cfg(feature = "search")]
#[allow(clippy::module_name_repetitions)]
/// Defines modes in which the search can run
pub enum SearchMode {
    /// Find matches from or after the current page
    Forward,
    /// Find matches before the current page
    Reverse,
    /// Don;t know the current search mode
    Unknown,
}

/// Fetch the search query asynchronously
#[cfg(all(feature = "static_output", feature = "search"))]
pub(crate) fn fetch_input_blocking(
    out: &mut impl std::io::Write,
    search_mode: SearchMode,
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
                    write!(out, "{}", cursor::Hide)?;
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
    search_mode: SearchMode,
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
pub(crate) fn highlight_search(pager: &mut Pager) {
    let pattern = pager.search_term.as_ref().unwrap();
    let mut coordinates: Vec<u16> = Vec::new();

    for (idx, line) in pager.get_flattened_lines().enumerate() {
        if pattern.is_match(&(*line).to_string()) {
            coordinates.push(u16::try_from(idx).unwrap());
        }
    }
    pager.search_idx = coordinates;
}

#[cfg(feature = "search")]
pub(crate) fn highlight_line_matches(line: &mut String, query: &regex::Regex) {
    use crossterm::style::Stylize;
    if let Some(cap) = query.captures(line) {
        let text = format!("{}{}{}", Attribute::Reverse, &cap[0], Attribute::NoReverse);
        let text = text.as_str();
        *line = query.replace_all(line, text).to_string();
    }
}

#[cfg(feature = "search")]
pub(crate) fn next_match(pager: &mut Pager, s_mark: &mut usize) {
    while let Some(y) = pager.search_idx.get(*s_mark) {
        if usize::from(*y) <= pager.upper_mark {
            *s_mark += 1;
        } else {
            pager.upper_mark = *y as usize;
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{highlight_line_matches, highlight_search, next_match};
    use crate::Pager;
    use crossterm::style::Attribute;
    use regex::Regex;

    #[test]
    fn test_next_match() {
        let mut pager = Pager::new().unwrap();
        let mut s_mark = 0;
        // A sample index for mocking actual search index matches
        pager.search_idx = vec![2, 10, 15, 17, 50];
        for i in &pager.search_idx.clone() {
            next_match(&mut pager, &mut s_mark);
            dbg!(pager.upper_mark);
            assert_eq!(pager.upper_mark, *i as usize);
        }
    }

    #[test]
    fn test_highlight_matches() {
        let mut line = "Integer placerat tristique nisl. placerat non mollis, magna orci dolor, placerat at vulputate neque nulla lacinia eros.".to_string();
        let pat = Regex::new(r"\W\w+t\W").unwrap();
        let result = format!(
            "Integer{inverse} placerat {noinverse}tristique nisl.\
{inverse} placerat {noinverse}non mollis, magna orci dolor,\
{inverse} placerat {noinverse}at vulputate neque nulla lacinia \
eros.",
            inverse = Attribute::Reverse,
            noinverse = Attribute::NoReverse
        );

        highlight_line_matches(&mut line, &pat);
        assert_eq!(line, result);
    }

    #[test]
    fn test_highlight_search() {
        let mut pager = Pager::new().unwrap();

        pager.set_text(
            "\
Fusce suscipit, wisi nec facilisis facilisis, est dui fermentum leo, quis tempor ligula 
erat quis odio.  Nunc porta vulputate tellus.  Nunc rutrum turpis sed pede.  Sed 
bibendum.  Aliquam posuere.  Nunc aliquet, augue nec adipiscing interdum, lacus tellus 
malesuada massa, quis varius mi purus non odio.  Pellentesque condimentum, magna ut 
suscipit hendrerit, ipsum augue ornare nulla, non luctus diam neque sit amet urna.  
Curabitur vulputate vestibulum lorem.  Fusce sagittis, libero non molestie mollis, magna 
orci ultrices dolor, at vulputate neque nulla lacinia eros.  Sed id ligula quis est 
convallis tempor.  Curabitur lacinia pulvinar nibh.  Nam a sapien.",
        );

        pager.search_term = Some(Regex::new(r"\Wa\w+\W").unwrap());
        let res = vec![3, 7, 11];
        highlight_search(&mut pager);
        assert_eq!(pager.search_idx, res);
    }
}
