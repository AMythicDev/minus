//! See the [`draw`] function exposed by this module.
use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::Attribute,
    terminal::{disable_raw_mode, Clear, ClearType, LeaveAlternateScreen},
};
use std::io::{prelude::*, stdout};
use std::time::Duration;
use std::{
    fmt::Write as _,
    io::{self, Write as _},
};

/// Events handled by the `minus` pager.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum InputEvent {
    /// `Ctrl+C` or `Q`, exits the application.
    Exit,
    /// The terminal was resized. Contains the new number of rows.
    UpdateRows(usize),
    /// `Up` or `Down` was pressed. Contains the new value for the upper mark.
    UpdateUpperMark(usize),
    /// `Ctrl+L`, inverts the line number display. Contains the new value.
    UpdateLineNumber(LineNumbers),
}

/// Returns the input corresponding to the given event, updating the data as
/// needed (`upper_mark`, `ln` or nothing).
///
/// - `upper_mark` will be (inc|dec)remented if the (`Up`|`Down`) is pressed.
/// - `ln` will be inverted if `can_change_ln` is `true` and one of the
///   `tokio_lib` and `async_std_lib` feature is active. See the `Not`
///   implementation for [`LineNumbers`] for more information.
pub(crate) fn handle_input(
    ev: Event,
    upper_mark: usize,
    ln: LineNumbers,
    can_change_ln: bool,
) -> Option<InputEvent> {
    match ev {
        Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(1))),
        Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(1))),
        Event::Resize(_, height) => Some(InputEvent::UpdateRows(height as usize)),
        Event::Key(KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
        }) if can_change_ln & cfg!(any(feature = "async_std_lib", feature = "tokio_lib")) => {
            Some(InputEvent::UpdateLineNumber(!ln))
        }
        Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        }) => Some(InputEvent::Exit),
        _ => None,
    }
}

/// Draws (at most) `rows` `lines`, where the first line to display is
/// `upper_mark`. This function will always try to display as much lines as
/// possible within `rows`.
///
/// If the total number of lines is less than `rows`, they will all be
/// displayed, regardless of `upper_mark` (which will be updated to reflect
/// this).
///
/// It will not wrap long lines.
pub(crate) fn draw(
    out: &mut impl io::Write,
    lines: &str,
    rows: usize,
    upper_mark: &mut usize,
    ln: LineNumbers,
) -> io::Result<()> {
    write!(out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    write_lines(out, lines, rows, upper_mark, ln)?;

    #[allow(clippy::cast_possible_truncation)]
    {
        write!(
            out,
            "{}{}Press q or Ctrl+C to quit{}",
            // `rows` is originally a u16, we got it from crossterm::terminal::size.
            MoveTo(0, rows as u16),
            Attribute::Reverse,
            Attribute::Reset,
        )?;
    }

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
pub(crate) fn write_lines(
    out: &mut impl io::Write,
    lines: &str,
    rows: usize,
    upper_mark: &mut usize,
    ln: LineNumbers,
) -> io::Result<()> {
    // '.count()' will necessarily finish since iterating over the lines of a
    // String cannot yield an infinite iterator, at worst a very long one.
    let line_count = lines.lines().count();

    // This will either do '-1' or '-0' depending on the lines having a blank
    // line at the end or not.
    let mut lower_mark = *upper_mark + rows - lines.ends_with('\n') as usize;

    if lower_mark > line_count {
        lower_mark = line_count;
        *upper_mark = if line_count < rows {
            0
        } else {
            line_count.saturating_sub(rows)
        };
    }

    let lines = lines
        .lines()
        .skip(*upper_mark)
        .take(lower_mark - *upper_mark);

    match ln {
        LineNumbers::AlwaysOff | LineNumbers::Disabled => {
            for line in lines {
                writeln!(out, "\r{}", line)?;
            }
        }
        LineNumbers::AlwaysOn | LineNumbers::Enabled => {
            let max_line_number = lower_mark + *upper_mark + 1;
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            {
                // Compute the length of a number as a string without allocating.
                //
                // While this may in theory lose data, it will only do so if
                // `max_line_number` is bigger than 2^52, which will probably
                // never happen. Let's worry about that only if someone reports
                // a bug for it.
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
    }

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

impl std::ops::Not for LineNumbers {
    type Output = Self;

    fn not(self) -> Self::Output {
        use LineNumbers::*;

        match self {
            Enabled => Disabled,
            Disabled => Enabled,
            ln => ln,
        }
    }
}

#[cfg(test)]
mod tests;
