use crossterm::{
    cursor::MoveTo,
    execute, queue,
    style::Attribute,
    terminal::{Clear, ClearType},
};

use std::{convert::TryInto, io::Write};

use super::term::move_cursor;
use crate::{error::MinusError, PagerState};

pub fn draw2(
    out: &mut impl Write,
    p: &mut PagerState,
    new_upper_mark: &mut usize,
) -> Result<(), MinusError> {
    let line_count = p.num_lines();

    // Reduce one row for prompt/messages
    //
    // NOTE This should be the value of rows that should be used throughout this function.
    // Don't use PagerState::rows, it might lead to wrong output
    let writable_rows = p.rows.saturating_sub(1);

    let delta = new_upper_mark.abs_diff(p.upper_mark);

    // Calculate the lower_bound for current and new upper marks
    // by adding either the rows or line_count depending on the minimality
    let lower_bound = p.upper_mark.saturating_add(writable_rows.min(line_count));
    let new_lower_bound = new_upper_mark.saturating_add(writable_rows.min(line_count));

    // If lower_mark is more than line_count, there could be two cases
    //
    // If the lower_bound is greater than the avilable line count, we set it to such a value
    // so that the last page can be displayed entirely, i.e never scroll past the last line
    if new_lower_bound > line_count {
        *new_upper_mark = line_count.saturating_sub(writable_rows);
    }

    // Sometimes the value of delta is too large that we can rather use the value of the writable rows to
    // achieve the same effect with better performance. This means that we have to less lines to the terminal
    //
    // Think of it like this:-
    // Let's say the current upper mark is at 100 and writable rows is 25. Now if there is a jump of 200th line,
    // then instead of writing 100 lines, we can just jump to the 200 line and display the next 25 lines from there on.
    //
    // Hence we use get the minimum of those for displaying
    //
    // NOTE that the large delta case may not always be true in case of scrolling down. Actually this method produces
    // wrong output if this is not the case hence we still rely on using lower bounds method. But for scrolling up, we
    // need this value whatever the value of delta be.
    let normalized_delta = delta.min(writable_rows);

    let lines = if *new_upper_mark > p.upper_mark {
        queue!(
            out,
            crossterm::terminal::ScrollUp(normalized_delta.try_into().unwrap())
        )?;
        // Move up the cursor one extra line to cleanup the old junk prompt
        move_cursor(
            out,
            0,
            p.rows
                .saturating_sub(normalized_delta + 1)
                .try_into()
                .unwrap(),
            false,
        )?;
        queue!(out, Clear(ClearType::CurrentLine))?;

        if normalized_delta < p.rows {
            p.get_flattened_lines_with_bounds(lower_bound, new_lower_bound)
        } else {
            p.get_flattened_lines_with_bounds(
                *new_upper_mark,
                new_upper_mark.saturating_add(normalized_delta),
            )
        }
    } else if *new_upper_mark < p.upper_mark {
        execute!(
            out,
            crossterm::terminal::ScrollDown(delta.try_into().unwrap())
        )?;
        move_cursor(out, 0, 0, false)?;

        p.get_flattened_lines_with_bounds(
            *new_upper_mark,
            new_upper_mark.saturating_add(normalized_delta),
        )
    } else {
        &[]
    };

    for line in lines {
        writeln!(out, "\r{}", line)?;
    }

    super::display::write_prompt(out, &p.displayed_prompt, p.rows.try_into().unwrap())?;
    out.flush()?;

    Ok(())
}

/// Draws the scrren
///
/// The function will first print out the lines. This is handled inside the [`write_lines`]
/// function.
///
/// Then it wil check if there is any message to display.
///     - If there is one, it will display it at the prompt site
///     - If there isn't one, it will display the prompt in place of it
pub fn draw(out: &mut impl Write, pager: &mut PagerState) -> Result<(), MinusError> {
    super::term::move_cursor(out, 0, 0, false)?;
    queue!(out, Clear(ClearType::All))?;

    write_lines(out, pager)?;

    let pager_rows: u16 = pager.rows.try_into().map_err(|_| MinusError::Conversion)?;

    write_prompt(out, &pager.displayed_prompt, pager_rows)?;

    out.flush().map_err(MinusError::Draw)
}

pub fn write_prompt(out: &mut impl Write, text: &str, rows: u16) -> Result<(), MinusError> {
    write!(
        out,
        "{mv}\r{rev}{prompt}{reset}",
        mv = MoveTo(0, rows),
        rev = Attribute::Reverse,
        prompt = text,
        reset = Attribute::Reset,
    )?;
    Ok(())
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
pub fn write_lines(out: &mut impl Write, pager: &mut PagerState) -> Result<(), MinusError> {
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
    write!(out, "\r{}", displayed_lines)?;
    Ok(())
}

#[cfg(test)]
mod tests;
