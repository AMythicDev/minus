//! Utilities that are used in both static and async display.
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    execute,
    style::Attribute,
    terminal::{self, Clear, ClearType},
};

use std::{
    convert::TryFrom,
    io::{self, Write as _},
};

use crate::{
    error::{CleanupError, SetupError},
    AlternateScreenPagingError, Pager,
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

/// Events handled by the `minus` pager.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum InputEvent {
    /// `Ctrl+C` or `Q`, exits the application.
    Exit,
    /// The terminal was resized. Contains the new number of rows.
    UpdateTermArea(usize, usize),
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
#[allow(clippy::too_many_lines)]
pub(crate) fn handle_input(ev: Event, pager: &Pager) -> Option<InputEvent> {
    match ev {
        // Scroll up by one.
        Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(
            pager.upper_mark.saturating_sub(1),
        )),

        // Scroll down by one.
        Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(
            pager.upper_mark.saturating_add(1),
        )),

        // Mouse scroll up/down
        Event::Mouse(MouseEvent::ScrollUp(_, _, _)) => Some(InputEvent::UpdateUpperMark(
            pager.upper_mark.saturating_sub(5),
        )),
        Event::Mouse(MouseEvent::ScrollDown(_, _, _)) => Some(InputEvent::UpdateUpperMark(
            pager.upper_mark.saturating_add(5),
        )),
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
            pager.upper_mark.saturating_sub(pager.rows - 1),
        )),
        Event::Key(KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(
            pager.upper_mark.saturating_add(pager.rows - 1),
        )),

        // Resize event from the terminal.
        Event::Resize(width, height) => {
            Some(InputEvent::UpdateTermArea(width as usize, height as usize))
        }
        // Switch line number display.
        Event::Key(KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
        }) => Some(InputEvent::UpdateLineNumber(!pager.line_numbers)),
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
            if pager.search_mode == SearchMode::Reverse {
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
            if pager.search_mode == SearchMode::Reverse {
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

    match pager.line_numbers {
        LineNumbers::AlwaysOff | LineNumbers::Disabled => {
            // Get the lines and display them
            let lines = pager.get_lines();
            let displayed_lines = lines
                .iter()
                .flatten()
                .skip(pager.upper_mark)
                .take(rows.min(line_count));
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
            // Compute the length of a number as a string without allocating.
            //
            // While this may in theory lose data, it will only do so if
            // `line_count` is bigger than 2^52, which will probably never
            // happen. Let's worry about that only if someone reports a bug
            // for it.
            let len_line_number = (line_count as f64).log10().floor() as usize + 1;
            #[cfg(feature = "search")]
            if !pager.search_term.is_empty() {
                let mut lines = pager.lines.clone();

                // Line space + single dot character + 1 space
                let padding = len_line_number + 3;
                // Rehighlight  the lines which may have got distorted due to line
                // numbers
                for (idx, line) in lines.iter_mut().enumerate() {
                    *line = textwrap::wrap(&line.join(" "), pager.cols.saturating_sub(padding))
                        .iter()
                        .map(|c| c.to_string())
                        .collect();
                    for mut row in line.iter_mut() {
                        crate::search::highlight_line_matches(&mut row, &pager.search_term)
                            .map_err(|e| AlternateScreenPagingError::SearchExpError(e.into()))?;

                        row.insert_str(
                            0,
                            &format!(
                                "\r {bold}{number: >len$}.{reset}",
                                bold = crossterm::style::Attribute::Bold,
                                number = idx + 1,
                                len = len_line_number,
                                reset = crossterm::style::Attribute::Reset,
                            ),
                        );
                    }
                }

                let displayed_lines = lines
                    .iter()
                    .flatten()
                    .skip(pager.upper_mark)
                    .take(rows.min(line_count));

                for line in displayed_lines {
                    writeln!(out, "\r{}", line)?;
                }
                return Ok(());
            }

            let mut numbered_lines =
                annotate_line_numbers(pager.lines.clone(), len_line_number, pager.cols);
            let displayed_lines = numbered_lines
                .iter_mut()
                .skip(pager.upper_mark)
                .take(rows.min(line_count));

            for line in displayed_lines {
                writeln!(out, "\r{}", line)?;
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

// fn get_total_line_count(lines: Vec<>)

// Uncomment these once utils::tests are ready for the new API
#[cfg(test)]
mod tests;

fn annotate_line_numbers(
    mut lines: Vec<Vec<String>>,
    len_line_number: usize,
    cols: usize,
) -> Vec<String> {
    let padding = len_line_number + 3;
    for (idx, line) in lines.iter_mut().enumerate() {
        *line = textwrap::wrap(&line.join(" "), cols.saturating_sub(padding))
            .iter()
            .map(|c| c.to_string())
            .collect();

        for row in line.iter_mut() {
            row.insert_str(
                0,
                &format!(
                    " {bold}{number: >len$}.{reset} ",
                    bold = crossterm::style::Attribute::Bold,
                    number = idx + 1,
                    len = len_line_number,
                    reset = crossterm::style::Attribute::Reset
                ),
            );
        }
    }

    lines.iter().flatten().map(|s| s.to_owned()).collect()
}
