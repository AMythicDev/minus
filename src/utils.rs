//! Utilities that are used in both static and async display.
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    execute,
    style::Attribute,
    terminal::{self, Clear, ClearType},
};

use std::io::{self, Write as _};

use crate::{
    error::{CleanupError, SetupError},
    Pager,
};

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
pub(crate) fn setup(stdout: &io::Stdout, dynamic: bool) -> std::result::Result<usize, SetupError> {
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

    let (_, rows) = terminal::size().map_err(|e| SetupError::TerminalSize(e.into()))?;

    Ok(rows as usize)
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
    pager: &Pager,
) -> std::result::Result<(), CleanupError> {
    // Reverse order of setup.
    execute!(out, event::DisableMouseCapture)
        .map_err(|e| CleanupError::DisableMouseCapture(e.into()))?;
    execute!(out, cursor::Show).map_err(|e| CleanupError::ShowCursor(e.into()))?;
    terminal::disable_raw_mode().map_err(|e| CleanupError::DisableRawMode(e.into()))?;

    let (_, rows) = terminal::size().map_err(|e| CleanupError::TerminalSize(e.into()))?;
    if pager.page_if_havent_overflowed || pager.get_lines().lines().count() < rows as usize {
        execute!(out, terminal::LeaveAlternateScreen)
            .map_err(|e| CleanupError::LeaveAlternateScreen(e.into()))?;
    }
    if *es == crate::ExitStrategy::ProcessQuit {
        std::process::exit(0);
    } else {
        Ok(())
    }
}

/// Events handled by the `minus` pager.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InputEvent {
    /// `Ctrl+C` or `Q`, exits the application.
    Exit,
    /// The terminal was resized. Contains the new number of rows.
    UpdateRows(usize),
    /// `Up` or `Down` was pressed. Contains the new value for the upper mark.
    /// Also sent by `g` or `G`, which behave like Vim: jump to top or bottom.
    UpdateUpperMark(usize),
    /// `Ctrl+L`, inverts the line number display. Contains the new value.
    UpdateLineNumber(LineNumbers),
    /// `/`, Searching for certain pattern of text
    #[cfg(feature = "search")]
    Search(SearchMode),
    /// Get to the next match in forward mode
    #[cfg(feature = "search")]
    NextMatch,
    /// Get to the previous match in forward mode
    #[cfg(feature = "search")]
    PrevMatch,
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

/// Returns the input corresponding to the given event, updating the data as
/// needed (`pager.upper_mark`, `pager.line_numbers` or nothing).
///
/// - `pager.upper_mark` will be (inc|dec)remented if the (`Up`|`Down`) is pressed.
/// - `pager.line_numbers` will be inverted if `Ctrl+L` is pressed. See the `Not` implementation
///   for [`LineNumbers`] for more information.
pub(crate) fn handle_input(
    ev: Event,
    upper_mark: usize,
    #[cfg(feature = "search")] search_mode: SearchMode,
    ln: LineNumbers,
    rows: usize,
) -> Option<InputEvent> {
    match ev {
        // Scroll up by one.
        Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(1))),

        // Scroll down by one.
        Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(1))),

        // Mouse scroll up/down
        Event::Mouse(MouseEvent::ScrollUp(_, _, _)) => {
            Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(5)))
        }
        Event::Mouse(MouseEvent::ScrollDown(_, _, _)) => {
            Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(5)))
        }
        // Go to top.
        Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(0)),
        // Go to bottom.
        Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::SHIFT,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::SHIFT,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(usize::MAX)),

        // Page Up/Down
        Event::Key(KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(
            upper_mark.saturating_sub(rows - 1),
        )),
        Event::Key(KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(
            upper_mark.saturating_add(rows - 1),
        )),

        // Resize event from the terminal.
        Event::Resize(_, height) => Some(InputEvent::UpdateRows(height as usize)),
        // Switch line number display.
        Event::Key(KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
        }) => Some(InputEvent::UpdateLineNumber(!ln)),
        // Quit.
        Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        }) => Some(InputEvent::Exit),
        #[cfg(feature = "search")]
        Event::Key(KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::Search(SearchMode::Forward)),
        #[cfg(feature = "search")]
        Event::Key(KeyEvent {
            code: KeyCode::Char('?'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::Search(SearchMode::Reverse)),
        #[cfg(feature = "search")]
        Event::Key(KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::NONE,
        }) => {
            if search_mode == SearchMode::Reverse {
                Some(InputEvent::PrevMatch)
            } else {
                Some(InputEvent::NextMatch)
            }
        }
        #[cfg(feature = "search")]
        Event::Key(KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
        }) => {
            if search_mode == SearchMode::Reverse {
                Some(InputEvent::NextMatch)
            } else {
                Some(InputEvent::PrevMatch)
            }
        }
        _ => None,
    }
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
pub(crate) fn draw(out: &mut impl io::Write, mut pager: &mut Pager, rows: usize) -> io::Result<()> {
    if !pager.page_if_havent_overflowed && pager.get_lines().lines().count() <= rows {
        return draw_without_paging(out, pager, rows);
    }
    write!(out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    // There must be one free line for the help message at the bottom.
    write_lines(out, &mut pager, rows.saturating_sub(1))?;

    #[allow(clippy::cast_possible_truncation)]
    {
        write!(
            out,
            "{mv}\r{rev}{prompt}{reset}",
            // `rows` is originally a u16, we got it from crossterm::terminal::size.
            mv = MoveTo(0, rows as u16),
            rev = Attribute::Reverse,
            prompt = pager.prompt,
            reset = Attribute::Reset,
        )?;
    }

    out.flush()
}

fn draw_without_paging(
    out: &mut impl io::Write,
    mut pager: &mut Pager,
    rows: usize,
) -> io::Result<()> {
    write!(out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;
    write_lines(out, &mut pager, rows)
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
    pager: &mut Pager,
    rows: usize,
) -> io::Result<()> {
    // Get the line
    let lines = pager.get_lines();
    let lines = lines.lines();
    // '.count()' will necessarily finish since iterating over the lines of a
    // String cannot yield an infinite iterator, at worst a very long one.
    let line_count = lines.clone().count();

    // This may be too high but the `Iterator::take` call below will limit this
    // anyway while allowing us to display as much lines as possible.
    let lower_mark = pager.upper_mark.saturating_add(rows.min(line_count));

    if lower_mark > line_count {
        pager.upper_mark = if line_count < rows {
            0
        } else {
            line_count.saturating_sub(rows)
        };
    }

    let displayed_lines = lines.skip(pager.upper_mark).take(rows.min(line_count));

    match pager.line_numbers {
        LineNumbers::AlwaysOff | LineNumbers::Disabled => {
            for line in displayed_lines {
                writeln!(out, "\r{}", line)?;
            }
        }
        LineNumbers::AlwaysOn | LineNumbers::Enabled => {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            {
                // Compute the length of a number as a string without allocating.
                //
                // While this may in theory lose data, it will only do so if
                // `line_count` is bigger than 2^52, which will probably never
                // happen. Let's worry about that only if someone reports a bug
                // for it.
                let len_line_number = (line_count as f64).log10().floor() as usize + 1;
                debug_assert_eq!(line_count.to_string().len(), len_line_number);

                for (idx, line) in displayed_lines.enumerate() {
                    writeln!(
                        out,
                        "\r{number: >len$}. {line}",
                        number = pager.upper_mark + idx + 1,
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
