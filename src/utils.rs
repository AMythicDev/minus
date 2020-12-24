//! Utilities that are used in both static and async display.
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    style::Attribute,
    terminal::{self, Clear, ClearType},
};
use std::sync::Arc;

use crate::error::{AlternateScreenPagingError, CleanupError, SetupError};
use crate::{Pager, PagerMutex};
use std::{
    io::{self, Write as _},
};

use crate::search;

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
fn setup(stdout: &io::Stdout, dynamic: bool) -> std::result::Result<usize, SetupError> {
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

    crossterm::execute!(out, terminal::EnterAlternateScreen)
        .map_err(|e| SetupError::AlternateScreen(e.into()))?;
    terminal::enable_raw_mode().map_err(|e| SetupError::RawMode(e.into()))?;
    crossterm::execute!(out, cursor::Hide).map_err(|e| SetupError::HideCursor(e.into()))?;
    crossterm::execute!(out, event::EnableMouseCapture)
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
fn cleanup(mut out: impl io::Write) -> std::result::Result<(), CleanupError> {
    // Reverse order of setup.
    crossterm::execute!(out, event::DisableMouseCapture)
        .map_err(|e| CleanupError::DisableMouseCapture(e.into()))?;
    crossterm::execute!(out, cursor::Show).map_err(|e| CleanupError::ShowCursor(e.into()))?;
    terminal::disable_raw_mode().map_err(|e| CleanupError::DisableRawMode(e.into()))?;
    crossterm::execute!(out, terminal::LeaveAlternateScreen)
        .map_err(|e| CleanupError::LeaveAlternateScreen(e.into()))?;
    Ok(())
}
#[cfg(feature = "static_output")]
pub(crate) fn static_paging(mut pager: Pager) -> Result<(), AlternateScreenPagingError> {
    // Setup terminal
    let mut out = io::stdout();
    let mut rows = setup(&out, false)?;
    loop {
        draw(&mut out, &mut pager, rows)?;

        // Check for events
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            // Get the events
            let input = handle_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                pager.upper_mark,
                pager.line_numbers,
                rows,
            );
            // Update any data that may have changed
            match input {
                None => continue,
                Some(InputEvent::Exit) => return Ok(cleanup(out)?),
                Some(InputEvent::UpdateRows(r)) => rows = r,
                Some(InputEvent::UpdateUpperMark(um)) => pager.upper_mark = um,
                Some(InputEvent::UpdateLineNumber(l)) => {
                    pager.line_numbers = l;
                }
                Some(InputEvent::Search) => {
                    fetch_input(&mut out, rows)?;
                }
            }
            draw(&mut out, &mut pager, rows)?;
        }
    }
}

/// Runs the pager in dynamic mode for the `PagerMutex`.
///
/// `get` is a function that will extract the Pager lock from the
/// `PageMutex`. `get` is only called when drawing, Therefore, it can be mutated the entire time, except while drawing
///
/// ## Errors
///
/// Setting/cleaning up the terminal can fail and IO to/from the terminal can
/// fail.
// #[cfg(any(feature = "async_std_lib", feature = "tokio_lib"))]
pub(crate) async fn dynamic_paging(
    p: &Arc<PagerMutex>,
) -> std::result::Result<(), AlternateScreenPagingError> {
    // Setup terminal
    let mut out = io::stdout();
    let mut rows = setup(&out, true)?;
    // Lat printed string
    let mut last_printed = String::new();
    let mut search_term = String::new();

    loop {
        // Get the lock, clone it and immidiately drop the lock
        let guard = p.lock().await;
        let mut pager = guard.clone();
        drop(guard);

        // If the last displayed text is not same as the original text, then redraw original text
        // TODO: IMPROVE THIS SECTION
        if pager.lines != last_printed {
            draw(&mut out, &mut pager, rows)?;
            last_printed = pager.lines.clone();
        }

        // Check for events
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            // Get the events
            let input = handle_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                pager.upper_mark,
                pager.line_numbers,
                rows,
            );
            // Lock the value again
            let mut lock = p.lock().await;
            // Update any data that may have changed
            match input {
                None => continue,
                Some(InputEvent::Exit) => {
                    cleanup(out)?;
                    if lock.exit_strategy == crate::ExitStrategy::ProcessQuit {
                        std::process::exit(0);
                    } else {
                        return Ok(());
                    }
                }
                Some(InputEvent::UpdateRows(r)) => rows = r,
                Some(InputEvent::UpdateUpperMark(um)) => lock.upper_mark = um,
                Some(InputEvent::UpdateLineNumber(l)) => {
                    lock.line_numbers = l;
                }
                Some(InputEvent::Search) => {
                    let string = search::fetch_input(&mut out, rows).await?;
                    if !string.is_empty() {
                        search::highlight_search(&mut lock, &string).map_err(|e| AlternateScreenPagingError::SearchExpError(e.into()))?;
                        search_term = string;
                        search::locate_match(&mut lock, &search_term, false);
                    }
                }
                Some(InputEvent::NextMatch) if !search_term.is_empty() => {
                    search::locate_match(&mut lock, &search_term, false);
                }
                Some(InputEvent::PrevMatch) if !search_term.is_empty() => {
                    search::locate_match(&mut lock, &search_term, true)
                }
                _ => {continue;}
            }
            
            let mut pager = lock.clone();
            draw(&mut out, &mut pager, rows)?;
            // Update the lock if pager has changed
            *lock = pager;
        }
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
    Search,
    /// Get to the next match
    NextMatch,
    /// Get to the previous match
    PrevMatch
}

/// Returns the input corresponding to the given event, updating the data as
/// needed (`pager.upper_mark`, `pager.line_numbers` or nothing).
///
/// - `pager.upper_mark` will be (inc|dec)remented if the (`Up`|`Down`) is pressed.
/// - `pager.line_numbers` will be inverted if `Ctrl+L` is pressed. See the `Not` implementation
///   for [`LineNumbers`] for more information.
pub(crate) fn handle_input(ev: Event, upper_mark: usize, ln: LineNumbers, rows: usize) -> Option<InputEvent> {
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
        Event::Key(KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::Search),
        Event::Key(KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::NextMatch),
        Event::Key(KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::PrevMatch),
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
    // '.count()' will necessarily finish since iterating over the lines of a
    // String cannot yield an infinite iterator, at worst a very long one.
    let line_count = pager.lines.lines().count();

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

    let displayed_lines = pager
        .lines
        .lines()
        .skip(pager.upper_mark)
        .take(rows.min(line_count));

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
