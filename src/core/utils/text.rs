//! Text related functions
//!
//! minus has a very intresting but simple text model that you must go through to understand how minus works.
//!
//! # Text Block
//! A text block in minus is just a bunch of text that may contain newlines (`\n`) between them.
//! [`PagerState::lines`] is nothing but just a giant text block.
//!
//! # Line
//! A line is text that must not contain any newlines inside it but may or may not end with a newline.
//! Don't confuse this with Rust's [Lines](std::str::Lines) which is similar to minus's Lines terminolagy but only
//! differs for the fact that they don't end with a newline. Although the Rust's Lines is heavily used inside minus
//! as an important building block.
//!
//! # Row
//! A row is part of a line that fits perfectly inside one row of terminal. Out of the three text types, only row
//! is dependent on the terminal conditions. If the terminal gets resized, each row will grow or shrink to hold
//! more or less text inside it.
//!
//! # Termination
//! # Termination of Line
//! A line is called terminated when it ends with a newline character, otherwise it is called unterminated.
//! You may ask why is this important? Because minus supports completing a line in multiple steps, if we don't care
//! whether a line is terminated or not, we won't know that the data coming right now is part of the current line or
//! it is for a new line.
//!
//! # Termination of block
//! A block is terminated if the last line of the block is terminated i.e it ends with a newline character.
//!
//! # Unterminated rows
//! It is 0 in most of the cases. The only case when it has a non-zero value is a line or block of text is unterminated
//! In this case, it is equal to the number of rows that the last line of the block or a the line occupied.
//! 
//! Whenever new data comes while a line or block is unterminated minus cleans up the number of unterminated rows 
//! on the terminal i.e the entire last line. Then it merges the incoming data to the last line and then reprints 
//! them on the terminal.
//!
//! Why this complex approach?  
//! Simple! printing an entire page on the terminal is slow and this approach allows minus to reprint only the 
//! parts that are required without having to redraw everything

use crate::LineNumbers;

#[cfg(feature = "search")]
use {crate::minus_core::search, std::collections::BTreeSet};

/// How should the incoming text be drawn on the screen
pub enum AppendStyle {
    /// Draw only the region that needs to change
    PartialUpdate(Vec<String>),

    /// Redraw the entire screen
    FullRedraw,
}

pub struct AppendOpts<'a> {
    /// Contains the incoming text data
    pub text: &'a str,
    /// This is Some when the last line inside minus's present data is unterminated. It contains the last
    /// line to be attached to the the incoming text
    pub attachment: Option<String>,
    /// Status of line numbers
    pub line_numbers: LineNumbers,
    /// This is equal to the number of lines in [`PagerState::lines`]. This basically tells what line 
    /// number the line will hold.
    pub lines_count: usize,
    /// This is equal to the number of lines in [`PagerState::formatted_lines`]. This is used to
    /// calculate the search index of the rows of the line.
    pub formatted_lines_count: usize,
    /// Number of digits that line numbers occupy.
    pub len_line_number: usize,
    /// Actual number of columns available for displaying
    pub cols: usize,
    /// Number of lines that are previously unterminated. It is only relevent when there is `attachment` text otherwise
    /// it should be 0.
    pub prev_unterminated: usize,
    /// Search term if a search is active
    #[cfg(feature = "search")]
    pub search_term: &'a Option<regex::Regex>,
}

/// Properties related to appending of incoming data
pub struct AppendResult {
    /// Formatted incoming lines
    pub lines: Vec<String>,
    /// Number of rows that are unterminated
    pub num_unterminated: usize,
    /// If search is active, this contains the indices where search matches in the incoming text have been found
    #[cfg(feature = "search")]
    pub append_search_idx: BTreeSet<usize>,
}

/// Makes the text that will be displayed.
pub fn make_append_str(mut opts: AppendOpts<'_>) -> AppendResult {
    // Tells whether the line should go on a new row or should it be appended to the last line
    // By default it is set to true, unless a last line i.e attachment is not None
    #[cfg(feature = "search")]
    let mut append = true;

    // Compute the text to be format
    let to_format = opts.attachment.as_ref().map_or_else(
        || opts.text.to_string(),
        |attached_text| {
            // If attachment is not none, merge both the lines into one for formatting
            // Also set append to false, as we are not pushing a new row but rather overwriting a already placed row
            // in the terminal
            let mut s = String::with_capacity(opts.text.len() + attached_text.len());
            s.push_str(&attached_text);
            s.push_str(opts.text);
            #[cfg(feature = "search")]
            {
                append = false;
            }
            s
        },
    );

    // This will get filled if there is an ongoing search
    #[cfg(feature = "search")]
    let mut append_search_idx = BTreeSet::new();

    let to_format_size = to_format.lines().count();
    let lines = to_format
        .lines()
        .enumerate()
        .map(|(idx, s)| (idx, s.to_string()))
        .collect::<Vec<(usize, String)>>();

    let mut fmtl = Vec::with_capacity(256);

    // Number of rows that have been formatted so far
    // Whenever a line is formatted, this will be incremented to te number of rows that the formatted line has occupied
    let mut formatted_row_count = 0;

    // To format the text we first split the line into three parts: first line, last line and middle lines.
    // Then we individually format each of these and finally join each of these components together to form
    // the entire line, which is ready to be inserted into PagerState::formatted_lines.
    // At any point, calling .len() on any of these gives the number of rows that the line has occupied on the screen.

    // We need to take care of first line as it can either be itself from the text, if append is true or it can be
    // attachment + first line from text, if append is false
    let mut first_line = formatted_line(
        &lines.first().unwrap().1,
        opts.len_line_number,
        opts.lines_count,
        opts.line_numbers,
        // Reduce formatted index by one if we we are overwriting the last line on the terminal
        #[cfg(feature = "search")]
        formatted_row_count,
        #[cfg(feature = "search")]
        &mut append_search_idx,
        opts.cols,
        #[cfg(feature = "search")]
        opts.search_term,
    );
    formatted_row_count += first_line.len();

    // Format all other lines except the first and last line
    let mut mid_lines = lines
        .iter()
        .skip(1)
        .take(lines.len().saturating_sub(2))
        .flat_map(|(idx, line)| {
            let fmt_line = formatted_line(
                line,
                opts.len_line_number,
                opts.lines_count + idx,
                opts.line_numbers,
                #[cfg(feature = "search")]
                formatted_row_count,
                #[cfg(feature = "search")]
                &mut append_search_idx,
                opts.cols,
                #[cfg(feature = "search")]
                opts.search_term,
            );
            formatted_row_count += fmt_line.len();
            return fmt_line;
        })
        .collect::<Vec<String>>();

    // Format the last line, only if first line and last line are different. We can check this
    // by seeing whether to_format_len is greater than 1
    let last_line = if to_format_size > 1 {
        Some(formatted_line(
            &lines.last().unwrap().1,
            opts.len_line_number,
            opts.lines_count + to_format_size -1,
            opts.line_numbers,
            #[cfg(feature = "search")]
            formatted_row_count,
            #[cfg(feature = "search")]
            &mut append_search_idx,
            opts.cols,
            #[cfg(feature = "search")]
            opts.search_term,
        ))
    } else {
        None
    };

    #[cfg(feature = "search")]
    {
        // NOTE: VERY IMPORTANT BLOCK TO GET PROPER SEARCH INDEX
        // Here is the current scenario: suppose you have text block like this (markers are present to denote where a 
        // new line begins).
        //
        // * This is line one row one
        //   This is line one row two
        //   This is line one row three
        // * This is line two row one
        //   This is line two row two
        //   This is line two row three
        //   This is line two row four
        //
        // and suppose a match is found at line 1 row 2 and line 2 row 4. So the index generated will be [1, 6].
        // Let's say this text block is going to be appended to [PagerState::formatted_lines] from index 23.
        // Now if directly append this generated index to [`PagerState::search_idx`], it will probably be wrong
        // as these numbers are *relative to current text block*. The actual search index should have been 24, 30.
        //
        // To fix this we basically add the number of items in [`PagerState::formatted_lines`].
        // But another issue arrives where if the text block is not appended but is rather a continuation of the
        // already present line: basically for situation where append = false. For this we need to subtract the
        // number of rows that the last line occupied since it is also getting reformatted. This can be easily
        // calculated by taking help of [`PagerState::unterminated`] or opts.prev_unterminated.
        //
        // We can simply decrement formatted_lines_count by prev_unterminated to get the effect
        opts.formatted_lines_count = opts.formatted_lines_count.saturating_sub(opts.prev_unterminated);
        append_search_idx = append_search_idx.iter().map(|i| opts.formatted_lines_count + i).collect();
    }

    // Calculate number of rows which are part of last line and are left unterminated  due to absense of \n
    let unterminated = if opts.text.ends_with('\n') {
        // If the last line ends with \n, then the line is complete so nothing is left as unterminated
        0
    } else if to_format_size > 1 {
        // If tthere are more than 1 line of text, get the last line's size and return it as unterminated
        last_line.as_ref().unwrap().len()
    } else {
        // If there is only one line, return the size of first line
        first_line.len()
    };

    fmtl.append(&mut first_line);
    fmtl.append(&mut mid_lines);
    if let Some(mut ll) = last_line {
        fmtl.append(&mut ll);
    }

    AppendResult {
        lines: fmtl,
        num_unterminated: unterminated,
        #[cfg(feature = "search")]
        append_search_idx,
    }
}

/// Formats the given `line`
///
/// - `line`: The line to format
/// - `line_numbers`: tells whether to format the line with line numbers.
/// - `len_line_number`: is the length of the number of lines in [`PagerState::lines`] as in a string.
///     For example, this will be 2 if number of lines in [`PagerState::lines`] is 50 and 3 if
///     number of lines in [`PagerState::lines`] is 500. This is used for calculating the padding
///     of each displayed line.
/// - `idx`: is the position index where the line is placed in [`PagerState::lines`].
/// - `formatted_idx`: is the position index where the line will be placed in the resulting
///    [`PagerState::formatted_lines`]
/// - `cols`: Number of columns in the terminal
/// - `search_term`: Contains the regex iif a search is active
pub(crate) fn formatted_line(
    line: &str,
    len_line_number: usize,
    idx: usize,
    line_numbers: LineNumbers,
    #[cfg(feature = "search")] formatted_idx: usize,
    #[cfg(feature = "search")] search_idx: &mut BTreeSet<usize>,
    cols: usize,
    #[cfg(feature = "search")] search_term: &Option<regex::Regex>,
) -> Vec<String> {
    assert!(!line.contains('\n'), "Newlines found in appending line {:?}", line);
    // Whether line numbers are active
    let line_numbers = matches!(line_numbers, LineNumbers::Enabled | LineNumbers::AlwaysOn);

    // NOTE: Only relevent when line numbers are active
    // Padding is the space that the actual line text will be shifted to accomodate for
    // line numbers. This is equal to:-
    // LineNumbers::EXTRA_PADDING + len_line_number
    //
    // We reduce this from the number of available columns as this space cannot be used for
    // actual line display when wrapping the lines
    let padding = len_line_number + LineNumbers::EXTRA_PADDING;

    // Wrap the line and return an iterator over all the rows
    let mut enumerated_rows = if line_numbers {
        wrap_str(line, cols.saturating_sub(padding + 2))
            .into_iter()
            .enumerate()
    } else {
        wrap_str(line, cols).into_iter().enumerate()
    };

    // highlight the lines with matching search terms
    // If a match is found, add this line's index to PagerState::search_idx
    // This is safe to call even if search feature is turned off or there is no search active: in these cases it wll
    // simply return the row
    let mut handle_search = |row: String, formatted_idx: usize, wrap_idx: usize| {
        #[cfg(feature = "search")]
        if let Some(st) = search_term.as_ref() {
            let (highlighted_row, is_match) = search::highlight_line_matches(&row, st);
            if is_match {
                search_idx.insert(formatted_idx + wrap_idx);
            }
            highlighted_row
        } else {
            row
        }
        #[cfg(not(feature = "search"))]
        row
    };

    if line_numbers {
        let mut formatted_rows = Vec::with_capacity(256);

        // Formatter for only when line numbers are active
        // * If minus is run under test, ascii codes for making the numbers bol is not inserted because they add
        // extra difficulty while writing tests
        // * Line number is added only to the first row of a line. This makes a better UI overall
        let formatter = |row: String, is_first_row: bool, idx: usize| {
            format!(
                "{bold}{number: >len$}{reset} {row}",
                bold = if cfg!(not(test)) && is_first_row {
                    crossterm::style::Attribute::Bold.to_string()
                } else {
                    "".to_string()
                },
                number = if is_first_row {
                    (idx + 1).to_string() + "."
                } else {
                    "".to_string()
                },
                len = padding,
                reset = if cfg!(not(test)) && is_first_row {
                    crossterm::style::Attribute::Reset.to_string()
                } else {
                    "".to_string()
                },
                row = row
            )
        };

        // First format the first row separate from other rows, then the subsequent rows and finally join them
        // This is because only the first row contains the line number and not the subsequent rows
        let first_row = {
            #[cfg_attr(not(feature = "search"), allow(unused_mut))]
            let mut row = enumerated_rows.next().unwrap().1.to_string();
            row = handle_search(row, formatted_idx, 0);
            formatter(row, true, idx)
        };
        formatted_rows.push(first_row);

        #[cfg_attr(not(feature = "search"), allow(unused_mut))]
        #[cfg_attr(not(feature = "search"), allow(unused_variables))]
        let mut rows_left = enumerated_rows
            .map(|(wrap_idx, mut row)| {
                row = handle_search(row, formatted_idx, wrap_idx);
                formatter(row, false, 0)
            })
            .collect::<Vec<String>>();
        formatted_rows.append(&mut rows_left);

        formatted_rows
    } else {
        // If line numbers aren't active, simply return the rows with search matches highlighted if search is active
        #[cfg_attr(not(feature = "search"), allow(unused_variables))]
        enumerated_rows
            .map(|(wrap_idx, row)| {
                handle_search(row, formatted_idx, wrap_idx)
            })
            .collect::<Vec<String>>()
    }
}

/// Wrap a line of string into a rows of a terminal based on the number of columns
/// Read the [`textwrap::wrap`] function for more info
pub(crate) fn wrap_str(line: &str, cols: usize) -> Vec<String> {
    textwrap::wrap(line, cols)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
}


#[cfg(test)]
mod unterminated {
    use super::make_append_str;
    use crate::PagerState;

    #[test]
    fn test_single_no_endline() {
        let ps = PagerState::new().unwrap();
        let append_style = make_append_str(&ps, "This is a line", None, 0, 0);
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_single_endline() {
        let ps = PagerState::new().unwrap();
        let append_style = make_append_str(&ps, "This is a line\n", None, 0, 0);
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_single_multi_newline() {
        let ps = PagerState::new().unwrap();
        let append_style = make_append_str(
            &ps,
            "This is a line\nThis is another line\nThis is third line",
            None,
            0,
            0,
        );
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_single_multi_endline() {
        let ps = PagerState::new().unwrap();
        let append_style =
            make_append_str(&ps, "This is a line\nThis is another line\n", None, 0, 0);
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_single_line_wrapping() {
        let mut ps = PagerState::new().unwrap();
        ps.cols = 20;
        let append_style = make_append_str(&ps, "This is a quite lengthy lint", None, 0, 0);
        assert_eq!(2, append_style.num_unterminated);
    }

    #[test]
    fn test_single_mid_newline_wrapping() {
        let mut ps = PagerState::new().unwrap();
        ps.cols = 20;
        let append_style = make_append_str(
            &ps,
            "This is a quite lengthy lint\nIt has three lines\nThis is
third line",
            None,
            0,
            0,
        );
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_single_endline_wrapping() {
        let mut ps = PagerState::new().unwrap();
        ps.cols = 20;
        let append_style = make_append_str(
            &ps,
            "This is a quite lengthy lint\nIt has three lines\nThis is
third line\n",
            None,
            0,
            0,
        );
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_no_endline() {
        let ps = PagerState::new().unwrap();
        let append_style = make_append_str(&ps, "This is a line", None, 0, 0);
        assert_eq!(1, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is another line",
            Some("This is a line".to_string()),
            1,
            append_style.num_unterminated,
        );
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_endline() {
        let ps = PagerState::new().unwrap();
        let append_style = make_append_str(&ps, "This is a line ", None, 0, 0);
        assert_eq!(1, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is another line\n",
            Some("This is a line ".to_string()),
            1,
            append_style.num_unterminated,
        );
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_multiple_newline() {
        let ps = PagerState::new().unwrap();
        let append_style = make_append_str(&ps, "This is a line\n", None, 0, 0);
        assert_eq!(0, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is another line\n",
            None,
            1,
            append_style.num_unterminated,
        );
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping() {
        let mut ps = PagerState::new().unwrap();
        ps.cols = 20;
        let append_style = make_append_str(&ps, "This is a line. This is second line", None, 0, 0);
        assert_eq!(2, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is another line\n",
            Some("This is a line. This is second line".to_string()),
            1,
            append_style.num_unterminated,
        );
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping_continued() {
        let mut ps = PagerState::new().unwrap();
        ps.cols = 20;
        let append_style =
            make_append_str(&ps, "This is a line. This is second line. ", None, 0, 0);
        assert_eq!(2, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is the third line",
            Some("This is a line. This is second line. ".to_string()),
            2,
            append_style.num_unterminated,
        );
        assert_eq!(3, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping_last_continued() {
        let mut ps = PagerState::new().unwrap();
        ps.cols = 20;
        let append_style =
            make_append_str(&ps, "This is a line.\nThis is second line. ", None, 0, 0);
        assert_eq!(1, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is the third line",
            Some("This is second line. ".to_string()),
            2,
            append_style.num_unterminated,
        );
        assert_eq!(3, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping_additive() {
        let mut ps = PagerState::new().unwrap();
        ps.cols = 20;
        let append_style = make_append_str(&ps, "This is a line.", None, 0, 0);
        assert_eq!(1, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is second line. ",
            Some("This is a line.".to_string()),
            1,
            append_style.num_unterminated,
        );
        assert_eq!(2, append_style.num_unterminated);
        let append_style = make_append_str(
            &ps,
            "This is third line",
            Some("This is a line.This is second line. ".to_string()),
            2,
            append_style.num_unterminated,
        );
        assert_eq!(3, append_style.num_unterminated);
    }
}

mod wrapping {
    // Test wrapping functions
    #[test]
    fn wrap_str() {
        let test = {
            let mut line = String::with_capacity(200);
            for _ in 1..=200 {
                line.push('#');
            }
            line
        };
        let result = wrap_str(&test, 80);
        assert_eq!(result.len(), 3);
        assert_eq!(
            (80, 80, 40),
            (result[0].len(), result[1].len(), result[2].len()),
        );
    }
}


