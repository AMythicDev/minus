//! See the [`draw`] function exposed by this module.
use crossterm::{
    cursor::MoveTo,
    style::Attribute,
    terminal::{Clear, ClearType},
};

use std::{
    fmt::Write as _,
    io::{self, Write as _},
};

const LINE_NUMBERS: LineNumbers = LineNumbers::No;

/// Draws (at most) `rows` `lines`, where the first line to display is
/// `upper_mark`. This function will always try to display as much lines as
/// possible within `rows`.
///
/// If the total number of lines is less than `rows`, they will all be
/// displayed, regardless of `upper_mark` (which will be updated to reflect
/// this).
///
/// It will no wrap long lines.
pub(crate) fn draw(lines: String, rows: usize, upper_mark: &mut usize) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Clear the screen and place cursor at the very top left.
    write!(&mut out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    write_lines(&mut out, &lines, rows, upper_mark, LINE_NUMBERS)?;

    // Display the prompt.
    write!(
        &mut out,
        "{}{}Press q or Ctrl+C to quit{}",
        MoveTo(0, rows as u16),
        Attribute::Reverse,
        Attribute::Reset,
    )?;

    out.flush()
}

/// Writes the given `lines` to the given `out`put.
///
/// - `rows` is the maximum number of lines to display at once.
/// - `upper_mark` is the index of the first line to display.
///
/// Lines should be separated by `\n` and `\r\n`.
///
/// No wrapping is done at all!
fn write_lines(
    out: &mut impl io::Write,
    lines: &str,
    rows: usize,
    upper_mark: &mut usize,
    numbers: LineNumbers,
) -> io::Result<()> {
    // '.count()' will necessarily finish since iterating over the lines of a
    // String cannot yield an infinite iterator, at worst a very long one.
    let line_count = lines.lines().count();

    // This will either do '-1' or '-0' depending on the lines having a blank
    // line at the end or not.
    let mut lower_mark = *upper_mark + rows - lines.ends_with('\n') as usize;

    // Do some necessary checking.
    // Lower mark should not be more than the length of lines vector.
    if lower_mark >= line_count {
        lower_mark = line_count;
        // If the length of lines is less than the number of rows, set upper_mark = 0
        *upper_mark = if line_count < rows {
            0
        } else {
            // Otherwise, set upper_mark to length of lines - rows.
            line_count - rows
        };
    }

    // Get the range of lines between upper mark and lower mark.
    let lines = lines.lines().skip(*upper_mark).take(lower_mark);

    match numbers {
        LineNumbers::No => {
            for line in lines {
                writeln!(out, "\r{}", line)?;
            }
        }
        LineNumbers::Yes => {
            let max_line_number = lower_mark + *upper_mark + 1;
            // Compute the length of a number as a string without allocating.
            let len_line_number = (max_line_number as f64).log10().floor() as usize + 1;
            debug_assert_eq!(max_line_number.to_string().len(), len_line_number);

            for (idx, line) in lines.enumerate() {
                writeln!(
                    out,
                    "\r{number: >len$}. {line}",
                    number = *upper_mark + idx + 1,
                    len = len_line_number,
                    line = line
                )?;
            }
        }
    }

    Ok(())
}

enum LineNumbers {
    Yes,
    No,
}

#[cfg(test)]
mod tests;
