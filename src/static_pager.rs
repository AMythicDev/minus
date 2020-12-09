//! Static information output, see [`page_all`].
use crate::{utils, Result};

use crossterm::{cursor, event, terminal};

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
pub fn page_all(lines: &str, mut ln: crate::LineNumbers) -> Result {
    let (_, rows) = terminal::size()?;
    let mut rows = rows as usize;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    // The upper mark of scrolling
    let mut upper_mark = 0;

    // If the number of lines in the output is less than the number of rows
    // then print it and exit the function.
    {
        let line_count = lines.lines().count();
        if rows > line_count {
            utils::write_lines(&mut out, lines, rows, &mut upper_mark, ln)?;
            out.flush()?;
            return Ok(());
        }
    }

    // Initialize the terminal
    crossterm::execute!(&mut out, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    crossterm::execute!(&mut out, cursor::Hide)?;

    loop {
        if event::poll(std::time::Duration::from_millis(10))? {
            use utils::InputEvent::*;

            let input = utils::handle_input(event::read()?, upper_mark, ln);
            match input {
                None => continue,
                Some(Exit) => {
                    crossterm::execute!(out, terminal::LeaveAlternateScreen)?;
                    terminal::disable_raw_mode()?;
                    crossterm::execute!(out, cursor::Show)?;
                    return Ok(());
                }
                Some(UpdateRows(r)) => rows = r,
                Some(UpdateUpperMark(um)) => upper_mark = um,
                Some(UpdateLineNumber(l)) => ln = l,
            };
        }

        utils::draw(&mut out, lines, rows, &mut upper_mark, ln)?;
    }
}
