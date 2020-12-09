//! Static information output, see [`page_all`].
use crate::{utils, Result};

use crossterm::terminal;

use std::io::{self, Write};

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
/// ```
/// use std::fmt::Write;
///
/// fn main() -> minus::Result<(), Box<dyn std::error::Error>> {
///     let mut output = String::new();
///
///     for i in 1..=30 {
///         writeln!(output, "{}", i)?;
///     }
///
///     minus::page_all(&output, minus::LineNumbers::Enabled)?;
///     Ok(())
/// }
/// ```
pub fn page_all(lines: &str, ln: crate::LineNumbers) -> Result {
    // If the number of lines in the output is less than the number of rows
    // then print it and exit the function.
    {
        let (_, rows) = terminal::size()?;
        let rows = rows as usize;
        let line_count = lines.lines().count();

        if rows > line_count {
            let stdout = io::stdout();
            let mut out = stdout.lock();
            utils::write_lines(&mut out, lines, rows, &mut 0, ln)?;
            out.flush()?;
            return Ok(());
        }
    }

    utils::alternate_screen_paging(ln, &lines, |l: &&str| *l)
}
