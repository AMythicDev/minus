use crossterm::{
    cursor::MoveTo,
    style::Attribute,
    terminal::{Clear, ClearType},
};

use std::{convert::TryInto, io::Write};

use crate::{MinusError, PagerState};

/// Draws the scrren
///
/// The function will first print out the lines. This is handled inside the [`write_lines`]
/// function.
///
/// Then it wil check if there is any message to display.
///     - If there is one, it will display it at the prompt site
///     - If there isn't one, it will display the prompt in place of it
pub fn draw(out: &mut impl Write, pager: &mut PagerState) -> Result<(), MinusError> {
    use crossterm::queue;

    super::term::move_cursor(out, 0, 0, false)?;
    queue!(out, Clear(ClearType::All))?;

    write_lines(out, pager)?;
    // If we have message, then show it or show the prompt text instead
    let prompt = pager
        .message
        .as_ref()
        .map_or_else(|| pager.prompt.clone(), std::clone::Clone::clone);

    let first_prompt = prompt.first().ok_or(MinusError::MissingSome)?;
    let pager_rows: u16 = pager.rows.try_into().map_err(|_| MinusError::Conversion)?;

    #[cfg(feature = "search")]
    if pager.search_idx.is_empty() {
        write_prompt(out, first_prompt, pager_rows)?;
    } else {
        //let search_text = format!("{}/{}", pager.search_mark + 1, pager.search_idx.len());
        let mut search_text = (pager.search_mark + 1).to_string();
        search_text.push('/');
        search_text.push_str(&pager.search_idx.len().to_string());

        let search_len = search_text.len();
        let prompt_str = if search_len + first_prompt.len() > pager.cols.saturating_sub(1) {
            &first_prompt[..pager.cols - search_len]
        } else {
            first_prompt
        };

        // use String::with_capacity here since this is called a lot and
        // we want to make sure it's as fast as possible
        let mut final_prompt = String::with_capacity(prompt_str.len() + 1 + search_text.len());
        final_prompt.push_str(prompt_str);
        final_prompt.push(' ');
        final_prompt.push_str(&search_text);

        write_prompt(out, &final_prompt, pager_rows)?;
    }


    // Prompt
    #[cfg(not(feature = "search"))]
    write_prompt(out, first_prompt, pager_rows)?;

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
