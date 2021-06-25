//! Utilities that are used in both static and async display.
use crossterm::{
    cursor::{self, MoveTo},
    event, execute,
    style::Attribute,
    terminal::{self, Clear, ClearType},
};

use std::{convert::TryFrom, io};

use crate::{
    error::{CleanupError, SetupError},
    AlternateScreenPagingError, Pager,
};

#[cfg(feature = "search")]
use crate::search::highlight_line_matches;

// This function should be kept close to `cleanup` to help ensure both are
// doing the opposite of the other.
/// Setup the terminal and get the necessary informations.
///
/// This will lock `stdout` for the lifetime of the pager.
///
/// `dynamic` tells whether `minus` will do a dynamic paging or static paging.
/// When `dynamic` is set to true, `minus` wll exit with an error if the stdout is nt
/// a TTY.
///
/// ## Errors
///
/// Setting up the terminal can fail, see [`SetupError`](SetupError).
pub(crate) fn setup(stdout: &io::Stdout, dynamic: bool) -> std::result::Result<(), SetupError> {
    let mut out = stdout.lock();

    // Check if the standard output is a TTY and not a file or something else but only in dynamic mode
    if dynamic {
        use crossterm::tty::IsTty;

        if out.is_tty() {
            Ok(())
        } else {
            Err(SetupError::InvalidTerminal)
        }?;
    }

    execute!(out, terminal::EnterAlternateScreen)
        .map_err(|e| SetupError::AlternateScreen(e.into()))?;
    terminal::enable_raw_mode().map_err(|e| SetupError::RawMode(e.into()))?;
    execute!(out, cursor::Hide).map_err(|e| SetupError::HideCursor(e.into()))?;
    execute!(out, event::EnableMouseCapture)
        .map_err(|e| SetupError::EnableMouseCapture(e.into()))?;
    Ok(())
}

/// Will try to clean up the terminal and set it back to its original state,
/// before the pager was setup and called.
///
/// Use this function if you encounter problems with your application not
/// correctly setting back the terminal on errors.
///
/// ## Errors
///
/// Cleaning up the terminal can fail, see [`CleanupError`](CleanupError).
pub(crate) fn cleanup(
    mut out: impl io::Write,
    es: &crate::ExitStrategy,
) -> std::result::Result<(), CleanupError> {
    // Reverse order of setup.
    execute!(out, event::DisableMouseCapture)
        .map_err(|e| CleanupError::DisableMouseCapture(e.into()))?;
    execute!(out, cursor::Show).map_err(|e| CleanupError::ShowCursor(e.into()))?;
    terminal::disable_raw_mode().map_err(|e| CleanupError::DisableRawMode(e.into()))?;
    execute!(out, terminal::LeaveAlternateScreen)
        .map_err(|e| CleanupError::LeaveAlternateScreen(e.into()))?;
    if *es == crate::ExitStrategy::ProcessQuit {
        std::process::exit(0);
    } else {
        Ok(())
    }
}

#[derive(PartialEq, Clone, Copy, Debug)]
#[cfg(feature = "search")]
/// Defines modes in which the search can run
pub enum SearchMode {
    /// Find matches from or after the current page
    Forward,
    /// Find matches before the current page
    Reverse,
    /// Don;t know the current search mode
    Unknown,
}

/// Draws (at most) `rows` `lines`, where the first line to display is
/// `pager.upper_mark`. This function will always try to display as much lines as
/// possible within `rows`.
///
/// If the total number of lines is less than `rows`, they will all be
/// displayed, regardless of `pager.upper_mark` (which will be updated to reflect
/// this).
///
/// It will not wrap long lines.

pub(crate) fn draw(
    out: &mut impl io::Write,
    mut pager: &mut Pager,
) -> Result<(), AlternateScreenPagingError> {
    write!(out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    // There must be one free line for the help message at the bottom.
    write_lines(out, &mut pager)?;

    // #[allow(clippy::cast_possible_truncation)]
    {
        write!(
            out,
            "{mv}\r{rev}{prompt}{reset}",
            // `rows` is originally a u16, we got it from crossterm::terminal::size.
            mv = MoveTo(0, u16::try_from(pager.rows).unwrap()),
            rev = Attribute::Reverse,
            prompt = pager.prompt,
            reset = Attribute::Reset,
        )?;
    }

    out.flush().map_err(AlternateScreenPagingError::Draw)
}

/// Writes the given `lines` to the given `out`put.
///
/// - `rows` is the maximum number of lines to display at once.
/// - `pager.upper_mark` is the index of the first line to display.
///
/// Lines should be separated by `\n` and `\r\n`.
///
/// No wrapping is done at all!
pub(crate) fn write_lines(
    out: &mut impl io::Write,
    mut pager: &mut Pager,
) -> Result<(), AlternateScreenPagingError> {
    let line_count = pager.num_lines();
    // Reduce one row for prompt
    let rows = pager.rows.saturating_sub(1);
    // This may be too high but the `Iterator::take` call below will limit this
    // anyway while allowing us to display as much lines as possible.
    let lower_mark = pager.upper_mark.saturating_add(rows.min(line_count));

    if lower_mark > line_count {
        pager.upper_mark = if line_count < pager.rows {
            0
        } else {
            line_count.saturating_sub(rows)
        };
    }

    let displayed_lines = match pager.line_numbers {
        LineNumbers::AlwaysOff | LineNumbers::Disabled => {
            // Get the unnested (flattened) lines and display them
            #[cfg_attr(not(feature = "search"), allow(unused_mut))]
            let mut lines = pager
                .get_flattened_lines()
                .skip(pager.upper_mark)
                .take(rows.min(line_count))
                .collect::<Vec<String>>();
            #[cfg(feature = "search")]
            if let Some(st) = &pager.search_term {
                for mut line in &mut lines {
                    highlight_line_matches(&mut line, st);
                }
            }
            lines
        }
        LineNumbers::AlwaysOn | LineNumbers::Enabled => {
            // Compute the length of a number as a string without allocating.
            //
            // While this may in theory lose data, it will only do so if
            // `line_count` is bigger than 2^52, which will probably never
            // happen. Let's worry about that only if someone reports a bug
            // for it.
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            let len_line_number = (line_count as f64).log10().floor() as usize + 1;
            annotate_line_numbers(
                pager.get_lines(),
                len_line_number,
                pager.cols,
                #[cfg(feature = "search")]
                &pager.search_term,
            )
            .iter()
            .skip(pager.upper_mark)
            .take(rows.min(line_count))
            .map(ToOwned::to_owned)
            .collect()
        }
    };
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

/// Add line numbers to all the lines taking into considerations the wraps
fn annotate_line_numbers(
    mut lines: Vec<Vec<String>>,
    len_line_number: usize,
    cols: usize,
    #[cfg(feature = "search")] search_term: &Option<regex::Regex>,
) -> Vec<String> {
    // Calculate the amount of space required for the numbering ie. length of line
    // numbers + . + 2 spaces and wrap according to it
    let padding = len_line_number + 3;
    for (idx, line) in lines.iter_mut().enumerate() {
        crate::rewrap(line, cols.saturating_sub(padding));

        // Insert the line numbers
        #[cfg_attr(not(feature = "search"), allow(unused_mut))]
        for mut row in line.iter_mut() {
            #[cfg(feature = "search")]
            if let Some(st) = search_term {
                // Highlight  the lines
                highlight_line_matches(&mut row, st);
            }
            // Make the formatted text
            // If function is called in a test run, reove the bold and reset
            // sequences because at that time we care more about correctness than
            // formatting
            let fmt_numbers = if cfg!(not(test)) {
                format!(
                    " {bold}{number: >len$}.{reset} ",
                    bold = crossterm::style::Attribute::Bold,
                    number = idx + 1,
                    len = len_line_number,
                    reset = crossterm::style::Attribute::Reset
                )
            } else {
                format!(
                    " {number: >len$}. ",
                    number = idx + 1,
                    len = len_line_number,
                )
            };
            // Insert line numbers at the beginning

            row.insert_str(0, &fmt_numbers);
        }
    }

    // Return the flattened lines
    lines.iter().flatten().map(ToOwned::to_owned).collect()
}

#[cfg(test)]
mod tests;
