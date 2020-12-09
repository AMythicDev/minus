//! See the [`draw`] function exposed by this module.
use crossterm::{
    cursor::MoveTo,
    cursor::Show,
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    execute,
    style::Attribute,
    terminal::{disable_raw_mode, Clear, ClearType, LeaveAlternateScreen},
};
use std::io::{prelude::*, stdout};
use std::time::Duration;
use std::{
    fmt::Write as _,
    io::{self, Write as _},
};

use crate::{Lines, Result};

// Behaviours on different events such as key presses, resizing
pub(crate) fn map_events(
    ln: &mut LineNumbers,
    mut upper_mark: &mut usize,
    rows: &mut usize,
    text: &str,
) -> Result {
    // Poll for keypresses
    if poll(Duration::from_millis(10)).unwrap() {
        match read().unwrap() {
            // If q or Ctrl+C is pressed, reset all changes to the terminal and quit
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                execute!(stdout(), LeaveAlternateScreen)?;
                disable_raw_mode()?;
                execute!(stdout(), Show)?;
                std::process::exit(0);
            }
            // Durng these events add a value to the upper_mark for moving down
            // Arrow Down - 1
            // Scrool Down - 5
            // Page Down - Number of rows -1
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            }) => {
                *upper_mark += 1;
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            Event::Key(KeyEvent {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => {
                *upper_mark += *rows - 1;
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            Event::Mouse(MouseEvent::ScrollDown(_, _, _)) => {
                *upper_mark += 5;
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            // Durng these events subtact a value to the upper_mark for moving up
            // Arrow Down - 1
            // Scrool Down - 5
            // Page Down - Number of rows -1
            Event::Key(KeyEvent {
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            }) => {
                *upper_mark = upper_mark.saturating_sub(1);
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            Event::Key(KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => {
                *upper_mark = upper_mark.saturating_sub(*rows - 1);
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            Event::Mouse(MouseEvent::ScrollUp(_, _, _)) => {
                *upper_mark = upper_mark.saturating_sub(5);
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            // When terminal is resized, update the rows and redraw
            Event::Resize(_, height) => {
                *rows = height as usize;
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            // Map Ctrl+L to enable line numbers
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::CONTROL,
            }) => {
                *ln = !*ln;
                draw(text, *rows, &mut upper_mark, *ln)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Draws (at most) `rows` `lines`, where the first line to display is
/// `upper_mark`. This function will always try to display as much lines as
/// possible within `rows`.
///
/// If the total number of lines is less than `rows`, they will all be
/// displayed, regardless of `upper_mark` (which will be updated to reflect
/// this).
///
/// It will no wrap long lines.
pub(crate) fn draw(
    lines: &str,
    rows: usize,
    upper_mark: &mut usize,
    ln: LineNumbers,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    // Clear the screen and place cursor at the very top left.
    write!(&mut out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    write_lines(&mut out, &lines, rows, upper_mark, ln)?;

    // Display the prompt.
    #[allow(clippy::cast_possible_truncation)]
    {
        write!(
            &mut out,
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
fn write_lines(
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

    // Do some necessary checking.
    // Lower mark should not be more than the length of lines vector.
    if lower_mark > line_count {
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
    let lines = lines
        .lines()
        .skip(*upper_mark)
        .take(lower_mark - *upper_mark);

    match ln {
        LineNumbers::No | LineNumbers::Disabled => {
            for line in lines {
                writeln!(out, "\r{}", line)?;
            }
        }
        LineNumbers::Yes | LineNumbers::Enabled => {
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

#[derive(PartialEq, Copy, Clone)]
pub enum LineNumbers {
    /// Enable line numbers permanenetly, cannot be turned off by user
    Enabled,
    /// Line numbers should be turned on, although users can turn it off
    Yes,
    /// Line numbers should be turned off, although users can turn it on
    No,
    /// Disable line numbers permanenetly, cannot be turned on by user
    Disabled,
}

impl std::ops::Not for LineNumbers {
    type Output = Self;

    fn not(self) -> Self::Output {
        if self == LineNumbers::Yes {
            LineNumbers::No
        } else if self == LineNumbers::No {
            LineNumbers::Yes
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests;
