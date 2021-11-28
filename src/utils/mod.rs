// Utilities that are used in both static and async display.
//
// The `term` module provide functions for setup/teardown of
// the terminal
pub(crate) mod ev_handler;
pub(crate) mod term;

use crossterm::{
    cursor::MoveTo,
    style::Attribute,
    terminal::{Clear, ClearType},
};

use std::{convert::TryFrom, io};

use crate::{AlternateScreenPagingError, Pager};

// Writes the given `lines` to the given `out`put.
//
// - `rows` is the maximum number of lines to display at once.
// - `pager.upper_mark` is the index of the first line to display.
//
// Lines should be separated by `\n` and `\r\n`.
//
// Draws (at most) `rows -1` `lines`, where the first line to display is
// `pager.upper_mark`. This function will always try to display as much lines as
// possible within `rows -1`.
//
// If the total number of lines is less than `rows -1`, they will all be
// displayed, regardless of `pager.upper_mark` (which will be updated to reflect
// this).
//
// Note that the last line is reserved for prompt and messages
pub(crate) fn draw(
    out: &mut impl io::Write,
    mut pager: &mut Pager,
) -> Result<(), AlternateScreenPagingError> {
    // If number of lines is less than number of rows and run_no_overflow is true, then write
    // the output and return
    //
    // No prompt to be displayed in this case
    if !pager.run_no_overflow && pager.num_lines() <= pager.rows {
        return write_lines(out, &mut pager);
    }
    write!(out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    write_lines(out, &mut pager)?;
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

    out.flush().map_err(AlternateScreenPagingError::Draw)
}

// Write the lines to the terminal
pub(crate) fn write_lines(
    out: &mut impl io::Write,
    mut pager: &mut Pager,
) -> Result<(), AlternateScreenPagingError> {
    let line_count = pager.num_lines();

    // Reduce one row for prompt
    let rows = pager.rows.saturating_sub(1);
    //
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

    let displayed_lines = pager.get_flattened_lines_with_bounds(pager.upper_mark, lower_mark);

    // Join the lines and display them at once
    // This is because, writing to console is slow
    //
    // Add \r to ensure cursor is placed at the beginning of each row
    writeln!(out, "\r{}", displayed_lines.join("\n\r"))?;

    Ok(())
}

/// Enum indicating whether to display the line numbers or not.
///
/// Note that displaying line numbers may be less performant than not doing it.
/// `minus` tries to do as quickly as possible but the numbers and padding
/// still have to be computed.
///
/// This implements [`Not`](std::ops::Not) to allow turning on/off line numbers
/// when they where not locked in by the binary displaying the text.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum LineNumbers {
    /// Enable line numbers permanently, cannot be turned off by user.
    AlwaysOn,
    /// Line numbers should be turned on, although users can turn it off
    /// (i.e, set it to `Disabled`).
    Enabled,
    /// Line numbers should be turned off, although users can turn it on
    /// (i.e, set it to `Enabled`).
    Disabled,
    /// Disable line numbers permanently, cannot be turned on by user.
    AlwaysOff,
}

impl LineNumbers {
    /// Returns `true` if `self` can be inverted (i.e, `!self != self`), see
    /// the documentation for the variants to know if they are invertible or
    /// not.
    #[allow(dead_code)]
    fn is_invertible(self) -> bool {
        matches!(self, Self::Enabled | Self::Disabled)
    }
}

impl std::ops::Not for LineNumbers {
    type Output = Self;

    fn not(self) -> Self::Output {
        use LineNumbers::{Disabled, Enabled};

        match self {
            Enabled => Disabled,
            Disabled => Enabled,
            ln => ln,
        }
    }
}

#[cfg(test)]
mod tests;
