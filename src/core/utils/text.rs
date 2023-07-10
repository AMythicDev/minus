#[cfg(feature = "search")]
use std::collections::BTreeSet;

use crate::PagerState;

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
    let mut first_line = p.formatted_line(
        &lines.first().unwrap().1,
        len_line_number,
        line_number_of_actual_placement,
        // Reduce formatted index by one if we we are overwriting the last line on the terminal
        #[cfg(feature = "search")]
        if append {
            line_number_of_actual_placement
        } else {
            line_number_of_actual_placement.saturating_sub(1)
        },
        #[cfg(feature = "search")]
        &mut append_search_idx,
    );

    // Format the last line, only if first line and last line are different. We can check this
    // by seeing whether to_format_len is greater than 1
    let last_line = if to_format_size > 1 {
        Some(p.formatted_line(
            &lines.last().unwrap().1,
            len_line_number,
            line_number_of_actual_placement + to_format_size,
            #[cfg(feature = "search")]
            line_number_of_actual_placement,
            #[cfg(feature = "search")]
            &mut append_search_idx,
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
            p.formatted_line(
                line,
                len_line_number,
                line_number_of_actual_placement + idx,
                #[cfg(feature = "search")]
                line_number_of_actual_placement,
                #[cfg(feature = "search")]
                &mut append_search_idx,
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
