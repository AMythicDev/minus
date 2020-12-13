//! Static information output, see [`page_all`].
use crate::utils;

use crossterm::terminal;
use crossterm::tty::IsTty;
use utils::AlternateScreenPagingError;

use std::io::{self, Write};

#[derive(Debug, thiserror::Error)]
pub enum PageAllError {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Paging(#[from] AlternateScreenPagingError),

    #[error("Failed to determine terminal size")]
    TerminalSize(crossterm::ErrorKind),
}

/// Outputs static information.
///
/// Once called, the `&str` passed to this function can never be changed. If you
/// want dynamic information:
///
#[cfg_attr(
    feature = "async_std_lib",
    doc = "- [`async_std_updating`](crate::async_std_updating)\n"
)]
#[cfg_attr(
    feature = "tokio_lib",
    doc = "- [`tokio_updating`](crate::tokio_updating)\n"
)]
#[cfg_attr(
    not(any(feature = "async_std_lib", feature = "tokio_lib")),
    doc = "- Asynchronous features are disabled, see [here](crate#features) for more information.\n"
)]
///
/// ## Errors
///
/// Several operations can fail when outputting information to a terminal, see
/// the [`Result`] type.
///
/// ## Example
///
/// ```rust,no_run
/// use std::fmt::Write;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let mut output = String::new();
///
///     for i in 0..=30 {
///         writeln!(output, "{}", i)?;
///     }
///
///     minus::page_all(&output, minus::LineNumbers::Enabled)?;
///     Ok(())
/// }
/// ```
pub fn page_all(lines: &str, ln: crate::LineNumbers) -> Result<(), PageAllError> {
    let stdout = io::stdout();
    let line_count = lines.lines().count();

    // If stdout is not a tty, print all the output without paging
    // then print it and exit the function.
    {
        if !stdout.is_tty() {
            let mut out = stdout.lock();
            utils::write_lines(&mut out, lines, line_count, &mut 0, ln)?;
            out.flush()?;
            return Ok(());
        }
    }

    {
        let (_, rows) = terminal::size().map_err(PageAllError::TerminalSize)?;
        let rows = rows as usize;

        // If the number of lines in the output is less than the number of rows
        if rows > line_count {
            let mut out = stdout.lock();
            utils::write_lines(&mut out, lines, rows, &mut 0, ln)?;
            out.flush()?;
        } else {
            utils::alternate_screen_paging(ln, &lines, |l: &&str| *l)?;
        }

        Ok(())
    }
}
