#![allow(dead_code)]

use crossterm::{
    cursor::MoveTo,
    execute, queue,
    style::Attribute,
    terminal::{Clear, ClearType},
};

use std::{cmp::Ordering, convert::TryInto, io::Write};

use super::{term, text::AppendStyle};
use crate::{error::MinusError, PagerState};

/// Handles drawing of screen based on movement
///
/// Refreshing the entire terminal can be costly, especially on high resolution displays and this cost can turns out to be
/// very high if that redrawing is required on every movement of the pager, even for small changes.
/// This function calculates what part of screen needs to be redrawed on scrolling up/down and based on that, it redraws
/// only that part of the terminal.
pub fn draw_for_change(
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

    // Calculate the lower_bound for current and new upper marks
    // by adding either the rows or line_count depending on the minimality
    let lower_bound = p.upper_mark.saturating_add(writable_rows.min(line_count));
    let new_lower_bound = new_upper_mark.saturating_add(writable_rows.min(line_count));

    // If the lower_bound is greater than the avilable line count, we set it to such a value
    // so that the last page can be displayed entirely, i.e never scroll past the last line
    if new_lower_bound > line_count {
        *new_upper_mark = line_count.saturating_sub(writable_rows);
    }

    let delta = new_upper_mark.abs_diff(p.upper_mark);
    // Sometimes the value of delta is too large that we can rather use the value of the writable rows to
    // achieve the same effect with better performance. This means that we have draw to less lines to the terminal
    //
    // Think of it like this:-
    // Let's say the current upper mark is at 100 and writable rows is 25. Now if there is a jump of 200th line,
    // then instead of writing 100 lines, we can just jump to the 200 line and display the next 25 lines from there on.
    //
    // Hence here we can take the minimum of the delta or writable rows for displaying
    //
    // NOTE that the large delta case may not always be true in case of scrolling down. Actually this method produces
    // wrong output if this is not the case hence we still rely on using lower bounds method. But for scrolling up, we
    // need this value whatever the value of delta be.
    let normalized_delta = delta.min(writable_rows);

    let lines = match (*new_upper_mark).cmp(&p.upper_mark) {
        Ordering::Greater => {
            // Scroll down `normalized_delta` lines, and put the cursor one line above, where the old prompt would present.
            // Clear it off and start displaying new dta.
            queue!(
                out,
                crossterm::terminal::ScrollUp(normalized_delta.try_into().unwrap())
            )?;
            term::move_cursor(
                out,
                0,
                p.rows
                    .saturating_sub(normalized_delta + 1)
                    .try_into()
                    .unwrap(),
                false,
            )?;
            queue!(out, Clear(ClearType::CurrentLine))?;

            if delta < writable_rows {
                p.get_formatted_lines_with_bounds(lower_bound, new_lower_bound)
            } else {
                p.get_formatted_lines_with_bounds(
                    *new_upper_mark,
                    new_upper_mark.saturating_add(normalized_delta),
                )
            }
        }
        Ordering::Less => {
            execute!(
                out,
                crossterm::terminal::ScrollDown(normalized_delta.try_into().unwrap())
            )?;
            term::move_cursor(out, 0, 0, false)?;

            p.get_formatted_lines_with_bounds(
                *new_upper_mark,
                new_upper_mark.saturating_add(normalized_delta),
            )
        }
        Ordering::Equal => return Ok(()),
    };

    for line in lines {
        writeln!(out, "\r{line}")?;
    }

    p.upper_mark = *new_upper_mark;

    super::display::write_prompt(out, &p.displayed_prompt, p.rows.try_into().unwrap())?;
    out.flush()?;

    Ok(())
}

/// Write given text at the prompt site
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

// The below functions are just a subset of functionality of the above draw_for_change function.
// Although, separate they are tightly coupled together.

/// Completely redraws the scrren
///
/// The function will first print out the lines from the current upper_mark. This is handled inside the [`write_lines`]
/// function.
///
/// Then it wil check if there is any message to display.
///   - If there is one, it will display it at the prompt site
///   - If there isn't one, it will display the prompt in place of it
pub fn draw_full(out: &mut impl Write, pager: &mut PagerState) -> Result<(), MinusError> {
    super::term::move_cursor(out, 0, 0, false)?;
    queue!(out, Clear(ClearType::All))?;

    write_from_pagerstate(out, pager)?;

    let pager_rows: u16 = pager.rows.try_into().map_err(|_| MinusError::Conversion)?;

    write_prompt(out, &pager.displayed_prompt, pager_rows)?;

    out.flush().map_err(MinusError::Draw)
}

pub fn draw_append_text(
    out: &mut impl Write,
    ps: &mut PagerState,
    prev_unterminated: usize,
    prev_fmt_lines_count: usize,
    append_style: AppendStyle,
) -> Result<(), MinusError> {
    if matches!(append_style, AppendStyle::FullRedraw) {
        draw_full(out, ps)?;
        return Ok(());
    }
    let AppendStyle::PartialUpdate(fmt_text) = append_style else {
        unreachable!()
    };

    if prev_fmt_lines_count < ps.rows {
        // Move the cursor to the very next line after the last displayed line
        term::move_cursor(
            out,
            0,
            prev_fmt_lines_count
                .saturating_sub(prev_unterminated)
                .try_into()
                .unwrap(),
            false,
        )?;
        // available_rows -> Rows that are still unfilled
        //      rows - number of lines displayed -1 (for prompt)
        // For example if 20 rows are in total in a terminal
        // and 10 rows are already occupied, then this will be equal to 9
        let available_rows = ps.rows.saturating_sub(
            prev_fmt_lines_count
                .saturating_sub(prev_unterminated)
                .saturating_add(1),
        );
        // Minimum amount of text that an be appended
        // If available_rows is less, than this will be available rows else it will be
        // the length of the formatted text
        //
        // If number of rows in terminal is 23 with 20 rows filled and another 5 lines are given
        // This woll be equal to 3 as available rows will be 3
        // If in the above example only 2 lines need to be added, this will be equal to 2
        let num_appendable = fmt_text.len().min(available_rows);
        if num_appendable >= 1 {
            crossterm::execute!(out, crossterm::terminal::Clear(ClearType::CurrentLine))?;
        }
        write!(out, "{}", fmt_text[0..num_appendable].join("\n\r"))?;
        out.flush()?;
    }
    Ok(())
}

/// Write the lines to the terminal
///
/// Note: Although this function can take any type that implements [Write] however it assumes that
/// it behaves like a terminal i.e it must set rows and cols in [PagerState].
/// If you want to write directly to a file without this preassumption, then use the [write_lines]
/// function.
///
/// Draws (at most) `rows -1` lines, where the first line to display is
/// [`PagerState::upper_mark`]. This function will always try to display as much lines as
/// possible within `rows -1`.
///
/// It always skips one row at the botton as a site for the prompt or any message that may be sent.
///
/// This function ensures that upper mark never exceeds a value such that adding upper mark and available rows exceeds
/// the number of lines of text data. This rule is disobeyed in only one special case which is if number of lines of
/// text is less than available rows. In this situation, upper mark is always 0.
pub fn write_text_checked(
    out: &mut impl Write,
    lines: &[String],
    rows: usize,
    upper_mark: &mut usize,
) -> Result<(), MinusError> {
    let line_count = lines.len();

    // Reduce one row for prompt/messages
    let writable_rows = rows.saturating_sub(1);

    // Calculate the lower_mark by adding either the rows or line_count depending
    // on the minimality
    let mut lower_mark = upper_mark.saturating_add(writable_rows.min(line_count));

    // If the lower_bound is greater than the avilable line count, we set it to such a value
    // so that the last page can be displayed entirely, i.e never scroll past the last line
    if lower_mark > line_count {
        *upper_mark = line_count.saturating_sub(writable_rows);
        lower_mark = upper_mark.saturating_add(writable_rows.min(line_count));
    }

    // Add \r to ensure cursor is placed at the beginning of each row
    let display_lines: &[String] = &lines[*upper_mark..lower_mark];

    term::move_cursor(out, 0, 0, false)?;
    term::clear_entire_screen(out, false)?;

    write_lines(out, display_lines, Some("\r"))
}

pub fn write_from_pagerstate(out: &mut impl Write, ps: &mut PagerState) -> Result<(), MinusError> {
    let line_count = ps.num_lines();

    // Reduce one row for prompt/messages
    let writable_rows = ps.rows.saturating_sub(1);

    // Calculate the lower_mark by adding either the rows or line_count depending
    // on the minimality
    let lower_mark = ps.upper_mark.saturating_add(writable_rows.min(line_count));

    // If the lower_bound is greater than the avilable line count, we set it to such a value
    // so that the last page can be displayed entirely, i.e never scroll past the last line
    if lower_mark > line_count {
        ps.upper_mark = line_count.saturating_sub(writable_rows);
    }

    // Add \r to ensure cursor is placed at the beginning of each row
    let display_lines: &[String] = ps.get_formatted_lines_with_bounds(ps.upper_mark, lower_mark);

    write_lines(out, display_lines, Some("\r"))
}

/// Write lines to the the output
///
/// Outputs all the `lines` to `out` without any preassumption about terminals.
/// `initial` tells any extra text to be inserted before each line. For functions that use this
/// function over terminals, this should be set to `\r` to avoid broken display.
/// The `\r` resets the cursor to the start of the line.
pub fn write_lines(
    out: &mut impl Write,
    lines: &[String],
    initial: Option<&str>,
) -> Result<(), MinusError> {
    for line in lines {
        writeln!(out, "{}{line}", initial.unwrap_or(""))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
