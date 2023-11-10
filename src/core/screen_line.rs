use crate::minus_core::utils::text::wrap_str;
use crate::LineNumbers;

pub struct ScreenLine {
    fmt_lines: Vec<String>,
    orig_text: String,
    line_number: usize,
    fmt_lines_count: usize,
    terminated: bool,
}

struct ScreenLineCreationOpts {
    text: String,
    cols: usize,
    line_number: usize,
    line_numbers: LineNumbers,
    len_line_numbers: u16,
    // prev_fmt_lines_count: usize,
}

impl ScreenLine {
    fn new(fmt_lines: Vec<String>, orig_text: String, line_number: usize) -> Self {
        let fmt_lines_count = fmt_lines.len();
        let terminated = orig_text.ends_with('\n');
        Self {
            fmt_lines,
            orig_text,
            line_number,
            fmt_lines_count,
            terminated,
        }
    }

    fn new_from_string(
        text: String,
        cols: u16,
        line_number: usize,
        line_numbers: LineNumbers,
        len_line_number: u16,
    ) -> Self {
        let fmt_lines = formatted_line(&text, len_line_number, line_number, line_numbers, cols);
        Self::new(fmt_lines, text, line_number)
    }
}

pub fn formatted_line(
    line: &str,
    len_line_number: u16,
    line_number: usize,
    line_numbers: LineNumbers,
    cols: u16,
) -> Vec<String> {
    assert!(
        !line.contains('\n'),
        "Newlines found in appending line {:?}",
        line
    );
    // Whether line numbers are active
    let line_numbers = matches!(line_numbers, LineNumbers::Enabled | LineNumbers::AlwaysOn);

    // NOTE: Only relevant when line numbers are active
    // Padding is the space that the actual line text will be shifted to accommodate for
    // line numbers. This is equal to:-
    // LineNumbers::EXTRA_PADDING + len_line_number + 1 (for '.')
    //
    // We reduce this from the number of available columns as this space cannot be used for
    // actual line display when wrapping the lines
    let padding = len_line_number + LineNumbers::EXTRA_PADDING + 1;

    // Wrap the line and return an iterator over all the rows
    let mut enumerated_rows = if line_numbers {
        wrap_str(line, cols.saturating_sub(padding + 2).into()).into_iter()
    } else {
        wrap_str(line, cols.into()).into_iter()
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
                    String::new()
                },
                number = if is_first_row {
                    (idx + 1).to_string() + "."
                } else {
                    String::new()
                },
                len = padding.into(),
                reset = if cfg!(not(test)) && is_first_row {
                    crossterm::style::Attribute::Reset.to_string()
                } else {
                    String::new()
                },
                row = row
            )
        };

        // First format the first row separate from other rows, then the subsequent rows and finally join them
        // This is because only the first row contains the line number and not the subsequent rows
        let first_row = {
            #[cfg_attr(not(feature = "search"), allow(unused_mut))]
            let mut row = enumerated_rows.next().unwrap();
            formatter(row, true, line_number)
        };
        formatted_rows.push(first_row);

        #[cfg_attr(not(feature = "search"), allow(unused_mut))]
        #[cfg_attr(not(feature = "search"), allow(unused_variables))]
        let mut rows_left = enumerated_rows
            .map(|mut row| formatter(row, false, 0))
            .collect::<Vec<String>>();
        formatted_rows.append(&mut rows_left);

        formatted_rows
    } else {
        enumerated_rows.collect()
    }
}
