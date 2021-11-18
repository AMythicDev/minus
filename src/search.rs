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
#[cfg(feature = "search")]
pub(crate) fn fetch_input(
    out: &mut impl std::io::Write,
    search_mode: SearchMode,
    rows: usize,
) -> Result<String, AlternateScreenPagingError> {
    // Place the cursor at the beginning of very prompt line, clear
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
                    write!(out, "{}", cursor::Hide)?;
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

// Set `Pager.search_idx` to the line numbers at which search matches are found
#[cfg(feature = "search")]
pub(crate) fn set_match_indices(pager: &mut Pager) {
    let pattern = match pager.search_term.as_ref() {
        Some(pat) => pat,
        None => return,
    };

    // Get all the lines in wrapping, check if they have a match
    // and put their line numbers if they do
    pager.search_idx = pager
        .formatted_lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            if pattern.is_match(line) {
                Some(idx)
            } else {
                None
            }
        })
        .collect();
}

#[cfg(feature = "search")]
pub(crate) fn highlight_line_matches(line: &str, query: &regex::Regex) -> String {
    use crossterm::style::Stylize;

    query
        .replace_all(line, |caps: &regex::Captures| {
            format!("{}{}{}", Attribute::Reverse, &caps[0], Attribute::NoReverse)
        })
        .to_string()
}

// Set variables to move to the next match
#[cfg(feature = "search")]
pub(crate) fn next_match(pager: &mut Pager, s_mark: &mut usize) {
    // Loop until we find a match, that's below the upper_mark
    //
    // Get match at the given mark
    while let Some(y) = pager.search_idx.get(*s_mark) {
        // If it's above upper_mark, continue for the next match
        if *y < pager.upper_mark {
            *s_mark += 1;
        } else {
            // If the condition is satisfied, set it and break
            pager.upper_mark = *y as usize;
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{highlight_line_matches, next_match, set_match_indices};
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
            s_mark += 1;
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

        assert_eq!(highlight_line_matches(&line, &pat), result);
    }

    #[test]
    fn test_set_match_indexes() {
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
        set_match_indices(&mut pager);
        assert_eq!(pager.search_idx, res);
    }
}
