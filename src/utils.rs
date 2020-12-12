//! Utilities that are used in both static and async display.
use crossterm::{
    cursor::{self, MoveTo},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    style::Attribute,
    terminal::{self, Clear, ClearType},
};

use anyhow::Context;
use std::io::{self, Write as _};

// This function should be kept close to `cleanup` to help ensure both are
// doing the opposite of the other.
/// Setup the terminal and get the necessary informations.
///
/// This will lock `stdout` for the lifetime of the pager.
///
/// ## Errors
///
/// Setting up the terminal can fail, see [`Result`](crate::Result).
fn setup(stdout: &io::Stdout) -> crate::Result<(io::StdoutLock<'_>, usize)> {
    let mut out = stdout.lock();

    // Check if the standard output is a TTY and not a file or somethng else but only in dynamic mode
    #[cfg(any(feature = "tokio_lib", feature = "anync_std_lib"))]
    {
        use crossterm::tty::IsTty;

        if out.is_tty() {
            Err(crate::Error::InvalidTerm)
        } else {
            Ok(())
        }?;
    }

    crossterm::execute!(out, terminal::EnterAlternateScreen)
        .context("Failed to switch to alternate screen")?;
    terminal::enable_raw_mode().context("Failed to enable raw mode")?;
    crossterm::execute!(out, cursor::Hide).context("Failed to hide the cursor")?;
    crossterm::execute!(out, event::EnableMouseCapture)
        .context("Failed to enable mouse capture")?;

    let (_, rows) = terminal::size().context("Couldn't determine the terminal size")?;

    Ok((out, rows as usize))
}

/// Will try to cleanup the terminal and set it back to its orignal state,
/// before the pager was setupped and called.
///
/// Use this function if you encounter problems with your application not
/// correctly setting back the terminal on errors.
///
/// ## Errors
///
/// Cleaning up the terminal can fail, see [`Result`](crate::Result).
pub fn cleanup(mut out: impl io::Write) -> crate::Result {
    // Reverse order of setup.
    crossterm::execute!(out, event::DisableMouseCapture)
        .context("Failed to disable mouse captureing terminal")?;
    crossterm::execute!(out, cursor::Show).context("Failed to show the cursor")?;
    terminal::disable_raw_mode().context("Failed to disable raw mode")?;
    crossterm::execute!(out, terminal::LeaveAlternateScreen)
        .context("Failed to switch back to main screen")?;
    Ok(())
}

/// Runs the pager for the given lines.
///
/// `get_lines` is a function that will extract the `&str` lines from the
/// storage type L. `get_lines` is only called when drawing, at the end of the
/// event loop. This means you can mutate the backing storage if it is an
/// `Arc<Mutex<String>>` for example, because the lock is not held for the
/// entire duration of the function, only for the drawing part.
///
/// See examples of usage in the `src/rt_wrappers.rs` and `src/static_pager.rs`
/// files.
///
/// ## Errors
///
/// Setting/cleaning up the terminal can fail and IO to/from the terminal can
/// fail.
pub(crate) fn alternate_screen_paging<L, F, S>(
    mut ln: LineNumbers,
    lines: &L,
    get_lines: F,
) -> crate::Result
where
    L: ?Sized,
    S: std::ops::Deref,
    S::Target: AsRef<str>,
    F: Fn(&L) -> S,
{
    let stdout = io::stdout();
    let (mut out, mut rows) =
        setup(&stdout).with_context(|| "Failed to initialize the terminal".to_string())?;
    // The upper mark of scrolling.
    let mut upper_mark = 0;
    let mut last_printed = String::new();

    loop {
        let lock = get_lines(lines);
        let string = lock.as_ref().to_string();
        drop(lock);

        if !string.eq(&last_printed) {
            draw(&mut out, &string, rows, &mut upper_mark, ln)
                .context("Failed to draw the new data")?;
            last_printed = string.clone();
        }

        if event::poll(std::time::Duration::from_millis(10))? {
            let input = handle_input(event::read()?, upper_mark, ln);

            match input {
                None => continue,
                Some(InputEvent::Exit) => return cleanup(out),
                Some(InputEvent::UpdateRows(r)) => rows = r,
                Some(InputEvent::UpdateUpperMark(um)) => upper_mark = um,
                Some(InputEvent::UpdateLineNumber(l)) => ln = l,
            }
            draw(&mut out, &string, rows, &mut upper_mark, ln)?;
        }
    }
}

/// Events handled by the `minus` pager.
#[derive(Debug, Copy, Clone, PartialEq)]
enum InputEvent {
    /// `Ctrl+C` or `Q`, exits the application.
    Exit,
    /// The terminal was resized. Contains the new number of rows.
    UpdateRows(usize),
    /// `Up` or `Down` was pressed. Contains the new value for the upper mark.
    /// Also sent by `g` or `G`, which behave like Vim: jump to top or bottom.
    UpdateUpperMark(usize),
    /// `Ctrl+L`, inverts the line number display. Contains the new value.
    UpdateLineNumber(LineNumbers),
}

/// Returns the input corresponding to the given event, updating the data as
/// needed (`upper_mark`, `ln` or nothing).
///
/// - `upper_mark` will be (inc|dec)remented if the (`Up`|`Down`) is pressed.
/// - `ln` will be inverted if `Ctrl+L` is pressed. See the `Not` implementation
///   for [`LineNumbers`] for more information.
fn handle_input(ev: Event, upper_mark: usize, ln: LineNumbers) -> Option<InputEvent> {
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
        // Mouse scroll up.
        Event::Mouse(MouseEvent::ScrollUp(_, _, _)) => {
            Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(5)))
        }
        // Mouse scroll down.
        Event::Mouse(MouseEvent::ScrollDown(_, _, _)) => {
            Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(5)))
        }
        // Go to top.
        Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE,
        })
        | Event::Key(KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(usize::MIN)),
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
        })
        | Event::Key(KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE,
        }) => Some(InputEvent::UpdateUpperMark(usize::MAX)),
        // Resize event from the terminal.
        Event::Resize(_, height) => Some(InputEvent::UpdateRows(height as usize)),
        // Switch line display.
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
fn draw(
    out: &mut impl io::Write,
    lines: &str,
    rows: usize,
    upper_mark: &mut usize,
    ln: LineNumbers,
) -> io::Result<()> {
    write!(out, "{}{}", Clear(ClearType::All), MoveTo(0, 0))?;

    // There must be one free line for the help message at the bottom.
    write_lines(out, lines, rows.saturating_sub(1), upper_mark, ln)?;

    #[allow(clippy::cast_possible_truncation)]
    {
        write!(
            out,
            "{mv}\r{rev}Press q or Ctrl+C to quit, g/G for top/bottom{lines}{reset}",
            // `rows` is originally a u16, we got it from crossterm::terminal::size.
            mv = MoveTo(0, rows as u16),
            rev = Attribute::Reverse,
            // Only display the help message when `Ctrl+L` will have an effect.
            lines = if ln.is_invertible() {
                ", Ctrl+L to display/hide line numbers"
            } else {
                ""
            },
            reset = Attribute::Reset,
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

    // This may be too high but the `Iterator::take` call below will limit this
    // anyway while allowing us to display as much lines as possible.
    let lower_mark = upper_mark.saturating_add(rows.min(line_count));

    if lower_mark > line_count {
        *upper_mark = if line_count < rows {
            0
        } else {
            line_count.saturating_sub(rows)
        };
    }

    let displayed_lines = lines.lines().skip(*upper_mark).take(rows.min(line_count));

    match ln {
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

impl LineNumbers {
    /// Returns `true` if `self` can be inverted (i.e, `!self != self`), see
    /// the documentation for the variants to know if they are invertible or
    /// not.
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
