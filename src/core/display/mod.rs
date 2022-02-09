use crossterm::{
    cursor::MoveTo,
    style::Attribute,
    terminal::{Clear, ClearType},
};

use std::{convert::TryFrom, io::Write};

use crate::{MinusError, PagerState};

/// Draws the scrren
///
/// The function will first print out the lines. This is handled inside the [`write_lines`]
/// function.
///
/// Then it wil check if there is any message to display.
///     - If there is one, it will display it at the prompt site
///     - If there isn't one, it will display the prompt in place of it
pub(crate) fn draw(out: &mut impl Write, pager: &mut PagerState) -> Result<(), MinusError> {
    write!(out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    write_lines(out, pager)?;
    // If we have message, then show it or show the prompt text instead
    let prompt = pager
        .message
        .0
        .as_ref()
        .map_or_else(|| pager.prompt.clone(), std::clone::Clone::clone);
    // Prompt
    {
        write!(
            out,
            "{mv}\r{rev}{prompt}{reset}",
            mv = MoveTo(0, u16::try_from(pager.rows).unwrap()),
            rev = Attribute::Reverse,
            prompt = prompt.first().unwrap(),
            reset = Attribute::Reset,
        )?;
    }

    out.flush().map_err(MinusError::Draw)
}

/// Write the lines to the terminal
///
/// Draws (at most) `rows -1` lines, where the first line to display is
/// [`PagerState::upper_mark`]. This function will always try to display as much lines as
/// possible within `rows -1`.
///
/// If the total number of lines is less than `rows -1`, they will all be
/// displayed, regardless of `pager.upper_mark` (which will be updated to reflect
/// this).
///
/// The function will always use `rows -1` lines as the the last line is reserved for prompt and messages
/// The lines are joined with "\n\r" since the terminal is in [raw
/// mode](../../crossterm/terminal/index.html#raw-mode). A "\n" takes the cursor directly below the
/// current line without taking it to the very begging, which is column 0.
/// Hence we use an additional "\r" to take the cursor to the very first column of the line.
pub(crate) fn write_lines(out: &mut impl Write, pager: &mut PagerState) -> Result<(), MinusError> {
    let line_count = pager.num_lines();

    // Reduce one row for prompt/messages
    let rows = pager.rows.saturating_sub(1);

    // Calculate the lower_mark by adding either the rows or line_count depending
    // on the minimality
    let lower_mark = pager.upper_mark.saturating_add(rows.min(line_count));

    // If lower_mark is more than line_count, there could be two cases
    if lower_mark > line_count {
        // There is not enough text to fill even a single page.
        // In this case, set upper_mark = 0
        pager.upper_mark = if line_count < pager.rows {
            0
        } else {
            // Else we set it to line_count - rows, equalling to the upper_mark of last page
            line_count.saturating_sub(rows)
        };
    }

    // Add \r to ensure cursor is placed at the beginning of each row
    let displayed_lines = pager
        .get_flattened_lines_with_bounds(pager.upper_mark, lower_mark)
        .join("\n\r");

    // Join the lines and display them at once
    // This is because, writing to console is slow
    writeln!(out, "\r{}", displayed_lines)?;
    Ok(())
}

#[cfg(test)]
mod tests;
