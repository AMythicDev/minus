//! Static information output, see [`page_all`].
use crate::utils::{draw, map_events};
use crate::Result;

use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::io::{stdout, Write};

/// Outputs static information.
///
/// Once called, the `&str` passed to this function can never be changed. If you
/// want dynamic information, see [`async_std_updating`] and [`tokio_updating`].
///
/// [`async_std_updating`]: crate::rt_wrappers::async_std_updating
/// [`tokio_updating`]: crate::rt_wrappers::tokio_updating
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
    // Get terminal rows and convert it to usize
    let (_, rows) = crossterm::terminal::size()?;
    let mut rows = rows as usize;

    // If the number of lines in the output is less than the number of rows
    // then print it and quit
    // FIXME(poliorcetics): use `draw` here for improved performance and avoid
    // code duplication.
    {
        let range: Vec<&str> = lines.split_terminator('\n').collect();
        if rows > range.len() {
            for line in range {
                println!("{}", line);
            }
            std::process::exit(0);
        }
    }

    // Initialize the terminal
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    execute!(stdout(), Hide)?;

    // The upper mark of scrolling
    let mut upper_mark = 0;

    // Draw at the very beginning
    draw(lines, rows, &mut upper_mark, ln)?;

    loop {
        map_events(&mut ln, &mut upper_mark, &mut rows, &lines)?;
    }
}
