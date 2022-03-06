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
use std::{convert::TryFrom, time::Duration};

static INVERT: Lazy<String> = Lazy::new(|| Attribute::Reverse.to_string());
static NORMAL: Lazy<String> = Lazy::new(|| Attribute::NoReverse.to_string());
static ANSI_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new("[\\u001b\\u009b]\\[[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]")
        .unwrap()
});

#[derive(Clone, Copy, Debug)]
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

/// Fetch the search query
///
/// The function will change the prompt to `/` for Forward search or `?` for Reverse search
/// It will then store the query in a String and return it when `Return` key is pressed
/// or return with a empty string if so match is found.
#[cfg(feature = "search")]
pub(crate) fn fetch_input(
    out: &mut impl std::io::Write,
    search_mode: SearchMode,
    rows: usize,
) -> Result<String, MinusError> {
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
        if event::poll(Duration::from_millis(10)).map_err(|e| MinusError::HandleEvent(e.into()))? {
            match event::read().map_err(|e| MinusError::HandleEvent(e.into()))? {
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

/// Set [`PagerState.search_idx`] to the line numbers at which search matches are found
///
/// The function will go through each line in [`PagerState::formatted_lines`] to check
/// if there is a search match. If a match is found, the function will append the index of the
/// string to [`PagerState::search_idx`]
pub(crate) fn set_match_indices(pager: &mut PagerState) {
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

/// Highlights the search match
pub(crate) fn highlight_line_matches(line: &str, query: &regex::Regex) -> String {
    // Remove all ansi escapes so we can look through it as if it had none
    let stripped_str = ANSI_REGEX.replace_all(line, "");

    // if it doesn't match, don't even try. Just return.
    if !query.is_match(&stripped_str) {
        return line.to_string();
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

    inverted
}

/// Set [`PagerState::search_mark`] to move to the next match
///
/// This function will continue looping untill it finds a match that is after the
/// [`PagerState::upper_mark`]
#[cfg(feature = "search")]
pub(crate) fn next_match(ps: &mut PagerState) {
    // Loop until we find a match, that's after the upper_mark
    //
    // Get match at the given mark
    while let Some(y) = ps.search_idx.get(ps.search_mark) {
        // If it's above upper_mark, continue for the next match
        if *y < ps.upper_mark {
            ps.search_mark += 1;
        } else {
            // If the condition is satisfied, set it and break
            ps.upper_mark = *y as usize;
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{highlight_line_matches, next_match, set_match_indices, INVERT, NORMAL};
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
        pager.search_idx = vec![2, 10, 15, 17, 50];
        for i in &pager.search_idx.clone() {
            next_match(&mut pager);
            assert_eq!(pager.upper_mark, *i as usize);
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

        assert_eq!(highlight_line_matches(&line, &pat), result);
    }

    #[test]
    fn test_set_match_indexes() {
        let mut pager = PagerState::new().unwrap();

        pager.lines = "\
Fusce suscipit, wisi nec facilisis facilisis, est dui fermentum leo, quis tempor ligula 
erat quis odio.  Nunc porta vulputate tellus.  Nunc rutrum turpis sed pede.  Sed 
bibendum.  Aliquam posuere.  Nunc aliquet, augue nec adipiscing interdum, lacus tellus 
malesuada massa, quis varius mi purus non odio.  Pellentesque condimentum, magna ut 
suscipit hendrerit, ipsum augue ornare nulla, non luctus diam neque sit amet urna.  
Curabitur vulputate vestibulum lorem.  Fusce sagittis, libero non molestie mollis, magna 
orci ultrices dolor, at vulputate neque nulla lacinia eros.  Sed id ligula quis est 
convallis tempor.  Curabitur lacinia pulvinar nibh.  Nam a sapien."
            .to_string();
        pager.format_lines();

        pager.search_term = Some(Regex::new(r"\Wa\w+\W").unwrap());
        let res = vec![3, 7, 11];
        set_match_indices(&mut pager);
        assert_eq!(pager.search_idx, res);
    }

    #[test]
    pub fn no_match() {
        let orig = "no match";
        let res = highlight_line_matches(orig, &Regex::new("test").unwrap());
        assert_eq!(res, orig.to_string());
    }

    #[test]
    pub fn single_match_no_esc() {
        let res = highlight_line_matches("this is a test", &Regex::new(" a ").unwrap());
        assert_eq!(res, format!("this is{} a {}test", *INVERT, *NORMAL));
    }

    #[test]
    pub fn multi_match_no_esc() {
        let res = highlight_line_matches("test another test", &Regex::new("test").unwrap());
        assert_eq!(
            res,
            format!("{i}test{n} another {i}test{n}", i = *INVERT, n = *NORMAL)
        );
    }

    #[test]
    pub fn esc_outside_match() {
        let res = highlight_line_matches(
            &format!("{}color{} and test", ESC, NONE),
            &Regex::new("test").unwrap(),
        );
        assert_eq!(
            res,
            format!("{}color{} and {}test{}", ESC, NONE, *INVERT, *NORMAL)
        );
    }

    #[test]
    pub fn esc_end_in_match() {
        let orig = format!("this {}is a te{}st", ESC, NONE);
        let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
        assert_eq!(res, format!("this {}is a {}test{}", ESC, *INVERT, *NORMAL));
    }

    #[test]
    pub fn esc_start_in_match() {
        let orig = format!("this is a te{}st again{}", ESC, NONE);
        let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
        assert_eq!(
            res,
            format!("this is a {}test{} again{}", *INVERT, *NORMAL, NONE)
        );
    }

    #[test]
    pub fn esc_around_match() {
        let orig = format!("this is {}a test again{}", ESC, NONE);
        let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
        assert_eq!(
            res,
            format!("this is {}a {}test{} again{}", ESC, *INVERT, *NORMAL, NONE)
        );
    }

    #[test]
    pub fn esc_within_match() {
        let orig = format!("this is a t{}es{}t again", ESC, NONE);
        let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
        assert_eq!(res, format!("this is a {}test{} again", *INVERT, *NORMAL));
    }

    #[test]
    pub fn multi_escape_match() {
        let orig = format!(
            "this {e}is a te{n}st again {e}yeah{n} test",
            e = ESC,
            n = NONE
        );
        let res = highlight_line_matches(&orig, &Regex::new("test").unwrap());
        assert_eq!(
            res,
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
