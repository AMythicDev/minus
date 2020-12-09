//! Dynamic information within a pager window.
//!
//! See [`tokio_updating`] and [`async_std_updating`] for more information.
use crate::utils::{draw, map_events};
use crate::LineNumbers;
use crate::{Lines, Result};

use crossterm::{
    cursor::Hide,
    event::EnableMouseCapture,
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};

use std::io::{prelude::*, stdout};
use std::time::Duration;

fn init(mutex: &Lines, mut ln: LineNumbers) -> Result {
    // Initialize the terminal
    execute!(stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    execute!(stdout(), Hide)?;
    execute!(stdout(), EnableMouseCapture)?;

    // Get terminal rows and convert it to usize
    let (_, rows) = crossterm::terminal::size().unwrap();
    let mut rows = rows as usize;
    // The upper mark of scrolling
    let mut upper_mark = 0;
    let mut last_printed = String::new();

    loop {
        // Lock the data and check errors
        let string = mutex.try_lock();
        if string.is_err() {
            continue;
        }
        // If no errors, compare it with the last displayed string
        // If they are not equal, display the new data
        let string = string.unwrap();
        // Use .eq() here as == cannot compare MutexGuard with a normal string
        if string.lines().count() < rows && !string.eq(&last_printed) {
            draw(&string, rows, &mut upper_mark, ln)?;
        }
        // Keap a copy of the string for later uee and drop it
        last_printed = string.to_string();
        drop(string);
        // Check for events asynchronously
        map_events(&mut ln, &mut upper_mark, &mut rows, &last_printed)?;
    }
}

/// Run the pager inside a [`tokio task`](tokio::task).
///
/// This function is only available when `tokio_lib` feature is enabled.
/// It takes a [`Lines`] and updates the page with new information when `Lines`
/// is updated.
///
/// This function switches to the [`Alternate Screen`] of the TTY and switches
/// to [`raw mode`].
///
/// ## Errors
///
/// Several operations can fail when outputting information to a terminal, see
/// the [`Result`] type.
///
/// ## Example
///
/// ```
/// use futures::join;
/// use tokio::time::sleep;
///
/// use std::fmt::Write;
/// use std::sync::{Arc, Mutex};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let output = Arc::new(Mutex::new(String::new()));
///
///     let increment = async {
///         let mut counter: u8 = 0;
///         while counter <= 30 {
///             let mut output = output.lock().unwrap();
///             writeln!(output, "{}", counter.to_string())?;
///             counter += 1;
///             drop(output);
///             sleep(Duration::from_millis(100)).await;
///         }
///         Result::<_, std::fmt::Error>::Ok(())
///     };
///
///     let (res1, res2) = join!(minus::tokio_updating(
///                output.clone(),
///                minus::LineNumbers::Yes), increment);
///     res1?;
///     res2?;
///     Ok(())
/// }
/// ```
///
/// **Please do note that you should never lock the output data, since this
/// will cause the paging thread to be paused. Only borrow it when it is
/// required and drop it if you have further asynchronous blocking code.**
///
/// [`Alternate Screen`]: crossterm::terminal#alternate-screen
/// [`raw mode`]: crossterm::terminal#raw-mode
#[cfg(feature = "tokio_lib")]
pub async fn tokio_updating(mutex: Lines, ln: LineNumbers) -> Result {
    use tokio::task;
    task::spawn(async move { init(&mutex, ln) }).await?
}

/// Initialize a updating pager inside an [`async_std task`].
///
/// This function is only available when `async_std_lib` feature is enabled
/// It takes a [`Lines`] and updates the page with new information when `Lines`
/// is updated.
///
/// This function switches to the [`Alternate Screen`] of the TTY and switches
/// to [`raw mode`].
///
/// ## Errors
///
/// Several operations can fail when outputting information to a terminal, see
/// the [`Result`] type.
///
/// ## Example
///
/// ```
/// use async_std::task::sleep;
/// use futures::join;
///
/// use std::fmt::Write;
/// use std::sync::{Arc, Mutex};
/// use std::time::Duration;
///
/// #[async_std::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let output = Arc::new(Mutex::new(String::new()));
///
///     let increment = async {
///         let mut counter: u8 = 0;
///         while counter <= 30 {
///             let mut output = output.lock().unwrap();
///             writeln!(output, "{}", counter.to_string())?;
///             counter += 1;
///             drop(output);
///             sleep(Duration::from_millis(100)).await;
///         }
///         Result::<_, std::fmt::Error>::Ok(())
///     };
///
///     let (res1, res2) = join!(minus::tokio_updating(
///                output.clone(),
///                minus::LineNumbers::Yes), increment);
///     res1?;
///     res2?;
///     Ok(())
/// }
/// ```
///
/// **Please do note that you should never lock the output data, since this
/// will cause the paging thread to be paused. Only borrow it when it is
/// required and drop it if you have further asynchronous blocking code.**
///
/// [`async_std task`]: async_std::task
/// [`Alternate Screen`]: crossterm::terminal#alternate-screen
/// [`raw mode`]: crossterm::terminal#raw-mode
#[cfg(feature = "async_std_lib")]
pub async fn async_std_updating(mutex: Lines, ln: LineNumbers) -> Result {
    use async_std::task;
    task::spawn(async move { init(&mutex, ln) }).await
}
