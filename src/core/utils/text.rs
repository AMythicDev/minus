use crate::{wrap_str, LineNumbers, PagerState};

#[cfg(feature = "search")]
use {crate::minus_core::search, std::collections::BTreeSet};

pub enum AppendStyle {
    PartialUpdate((Vec<String>, usize)),
    FullRedraw(usize),
}

pub struct AppendProps {
    pub lines: Vec<String>,
    pub num_unterminated: usize,
    #[cfg(feature = "search")]
    pub append_search_idx: BTreeSet<usize>,
}

/// Makes the text that will be displayed and appended it to [`self.formatted_lines`]
pub fn make_append_str(
    p: &PagerState,
    text: &str,
    attachment: Option<String>,
    line_number_of_actual_placement: usize,
    len_line_number: usize,
) -> AppendProps {
    // Tells whether the line should go on a new row or should it be appended to the last line
    // By default it is set to true, unless a last line i.e attachment is not None
    #[cfg(feature = "search")]
    let mut append = true;

    // Compute the text to be format
    let to_format = attachment.map_or_else(
        || text.to_string(),
        |attached_text| {
            // If attachment is not none, merge both the lines into one for formatting
            // Also set append to false, as we are not pushing a new row but rather overwriting a already placed row
            // in the terminal
            let mut s = String::with_capacity(text.len() + attached_text.len());
            s.push_str(&attached_text);
            s.push_str(text);
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

    // To format the text we first split the line into three parts: first line, last line and middle lines.
    // Then we individually format each of these and finally join each of these components together to form
    // the entire line, which is ready to be inserted into PagerState::formatted_lines.
    // At any point, calling .len() on any of these gives the number of rows that the line has occupied on the screen.

    // Here first line can just be
    // We need to take care of first line as it can either be itself from the text, if append is true or it can be
    // attachment + first line from text, if append is false
    let mut first_line = formatted_line(
        &lines.first().unwrap().1,
        len_line_number,
        line_number_of_actual_placement,
        p.line_numbers,
        // Reduce formatted index by one if we we are overwriting the last line on the terminal
        #[cfg(feature = "search")]
        if append {
            line_number_of_actual_placement
        } else {
            line_number_of_actual_placement.saturating_sub(1)
        },
        #[cfg(feature = "search")]
        &mut append_search_idx,
        p.cols,
        #[cfg(feature = "search")]
        &p.search_term,
    );

    // Format the last line, only if first line and last line are different. We can check this
    // by seeing whether to_format_len is greater than 1
    let last_line = if to_format_size > 1 {
        Some(formatted_line(
            &lines.last().unwrap().1,
            len_line_number,
            line_number_of_actual_placement + to_format_size,
            p.line_numbers,
            #[cfg(feature = "search")]
            line_number_of_actual_placement,
            #[cfg(feature = "search")]
            &mut append_search_idx,
            p.cols,
            #[cfg(feature = "search")]
            &p.search_term,
        ))
    } else {
        None
    };

    // Format all other lines except the first and last line
    let mut mid_lines = lines
        .iter()
        .skip(1)
        .take(lines.len().saturating_sub(2))
        .flat_map(|(idx, line)| {
            formatted_line(
                line,
                len_line_number,
                line_number_of_actual_placement + idx,
                p.line_numbers,
                #[cfg(feature = "search")]
                line_number_of_actual_placement,
                #[cfg(feature = "search")]
                &mut append_search_idx,
                p.cols,
                #[cfg(feature = "search")]
                &p.search_term,
            )
        })
        .collect::<Vec<String>>();

    // Calculate number of rows which are part of last line and are left unterminated  due to absense of \n
    let unterminated = if text.ends_with('\n') {
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

    AppendProps {
        lines: fmtl,
        num_unterminated: unterminated,
        #[cfg(feature = "search")]
        append_search_idx,
    }
}

/// Formats the given `line`
///
/// - `line_numbers` tells whether to format the line with line numbers.
/// - `len_line_number` is the length of the number of lines in [`PagerState::lines`] as in a string.
///     For example, this will be 2 if number of lines in [`PagerState::lines`] is 50 and 3 if
///     number of lines in [`PagerState::lines`] is 500. This is used for calculating the padding
///     of each displayed line.
/// - `idx` is the position index where the line is placed in [`PagerState::lines`].
/// - `formatted_idx` is the position index where the line will be placed in the resulting
///    [`PagerState::formatted_lines`]
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
    let line_numbers = matches!(line_numbers, LineNumbers::Enabled | LineNumbers::AlwaysOn);

    let padding = len_line_number + LineNumbers::EXTRA_PADDING;

    let mut enumerated_rows = if line_numbers {
        wrap_str(line, cols.saturating_sub(padding + 2))
            .into_iter()
            .enumerate()
    } else {
        wrap_str(line, cols).into_iter().enumerate()
    };

    // highlight the lines with matching search terms
    // If a match is found, add this line's index to PagerState::search_idx
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
        // Padding is the space that the actual line text will be shifted to accomodate for
        // in line numbers. This is equal to:-
        // 1 for initial space + len_line_number + 1 for `.` sign and + 1 for the followup space
        //
        // We reduce this from the number of available columns as this space cannot be used for
        // actual line display when wrapping the lines
        let mut formatted_rows = Vec::with_capacity(256);

        let formatter = |row: String, is_first_line: bool, idx: usize| {
            format!(
                "{bold}{number: >len$}.{reset} {row}",
                bold = if cfg!(not(test)) && is_first_line {
                    crossterm::style::Attribute::Bold.to_string()
                } else {
                    "".to_string()
                },
                number = if is_first_line {
                    (idx + 1).to_string()
                } else {
                    "".to_string()
                },
                len = padding,
                reset = if cfg!(not(test)) && is_first_line {
                    crossterm::style::Attribute::Reset.to_string()
                } else {
                    "".to_string()
                },
                row = row
            )
        };

        let first_line = {
            #[cfg_attr(not(feature = "search"), allow(unused_mut))]
            let mut row = enumerated_rows.next().unwrap().1.to_string();

            row = handle_search(row, formatted_idx, 0);

            formatter(row, true, idx + 1)
            // if cfg!(not(test)) {
            //     // format!(
            //     //     "{bold}{number: >len$}.{reset} {row}",
            //     //     bold = crossterm::style::Attribute::Bold,
            //     //     number = idx + 1,
            //     //     len = padding,
            //     //     reset = crossterm::style::Attribute::Reset,
            //     //     row = row
            //     // )
            //
            // } else {
            //     // In tests, we don't care about ANSI sequences for cool looking line numbers
            //     // hence we don't include them in tests. It just makes testing more difficult
            //     format!(
            //         "{number: >len$}. {row}",
            //         number = idx + 1,
            //         len = padding,
            //         row = row
            //     )
            // }
        };

        formatted_rows.push(first_line);

        #[cfg_attr(not(feature = "search"), allow(unused_mut))]
        #[cfg_attr(not(feature = "search"), allow(unused_variables))]
        let mut lines_left = enumerated_rows
            .map(|(wrap_idx, mut row)| {
                row = handle_search(row, formatted_idx, wrap_idx);
                formatter(row, false, 0)
                //" ".repeat(padding + 2) + &row
            })
            .collect::<Vec<String>>();
        formatted_rows.append(&mut lines_left);
        formatted_rows
    } else {
        #[cfg_attr(not(feature = "search"), allow(unused_variables))]
        enumerated_rows
            .map(|(wrap_idx, row)| {
                handle_search(row, formatted_idx, wrap_idx)
                // #[cfg(feature = "search")]
                // {
                //     search_term.as_ref().map_or_else(
                //         || row.to_string(),
                //         |st| handle_search(row.to_string(), formatted_idx, wrap_idx),
                //     )
                // }
                // #[cfg(not(feature = "search"))]
                // row.to_string()
            })
            .collect::<Vec<String>>()
    }
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
