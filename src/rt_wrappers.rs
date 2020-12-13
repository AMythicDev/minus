//! Dynamic information within a pager window.
//!
//! See [`tokio_updating`] and [`async_std_updating`] for more information.
use utils::AlternateScreenPagingError;

use crate::{utils, LineNumbers};

use std::sync::{Arc, Mutex};

/// An atomically reference counted string of all output for the pager.
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
pub type Lines = Arc<Mutex<String>>;

/// Run the pager inside a [`tokio task`](tokio::task).
///
/// This function is only available when `tokio_lib` feature is enabled.
/// It takes a [`Lines`] and updates the page with new information when `Lines`
/// is updated.
///
/// This function switches to the [`Alternate Screen`] of the TTY and switches
/// to [`raw mode`].
///
/// [`Alternate Screen`]: crossterm::terminal#alternate-screen
/// [`raw mode`]: crossterm::terminal#raw-mode
///
/// ## Errors
///
/// Several operations can fail when outputting information to a terminal, see
/// the [`Result`] type.
///
/// ## Example
///
/// ```rust,no_run
/// use futures::join;
/// use tokio::time::sleep;
///
/// use std::fmt::Write;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let output = minus::Lines::default();
///
///     let increment = async {
///         for i in 0..=30_u32 {
///             let mut output = output.lock().unwrap();
///             writeln!(output, "{}", i)?;
///             drop(output);
///             sleep(Duration::from_millis(100)).await;
///         }
///         Result::<_, std::fmt::Error>::Ok(())
///     };
///
///     let (res1, res2) = join!(
///         minus::tokio_updating(minus::Lines::clone(&output), minus::LineNumbers::Disabled),
///         increment
///     );
///     res1?;
///     res2?;
///     Ok(())
/// }
/// ```
///
/// **Please do note that you should never lock the output data, since this
/// will cause the paging thread to be paused. Only borrow it when it is
/// required and drop it if you have further asynchronous blocking code.**
#[cfg(feature = "tokio_lib")]
pub async fn tokio_updating(
    mutex: Lines,
    ln: LineNumbers,
) -> Result<(), AlternateScreenPagingError> {
    tokio::task::spawn(async move { run(&mutex, ln) }).await?
}

/// Run the pager inside an [`async_std task`](async_std::task).
///
/// This function is only available when `async_std_lib` feature is enabled
/// It takes a [`Lines`] and updates the page with new information when `Lines`
/// is updated.
///
/// This function switches to the [`Alternate Screen`] of the TTY and switches
/// to [`raw mode`].
///
/// [`Alternate Screen`]: crossterm::terminal#alternate-screen
/// [`raw mode`]: crossterm::terminal#raw-mode
///
/// ## Errors
///
/// Several operations can fail when outputting information to a terminal, see
/// the [`Result`] type.
///
/// ## Example
///
/// ```rust,no_run
/// use async_std::task::sleep;
/// use futures::join;
///
/// use std::fmt::Write;
/// use std::time::Duration;
///
/// #[async_std::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let output = minus::Lines::default();
///
///     let increment = async {
///         for i in 0..=30_u32 {
///             let mut output = output.lock().unwrap();
///             writeln!(output, "{}", i)?;
///             drop(output);
///             sleep(Duration::from_millis(100)).await;
///         }
///         Result::<_, std::fmt::Error>::Ok(())
///     };
///
///     let (res1, res2) = join!(
///         minus::async_std_updating(minus::Lines::clone(&output), minus::LineNumbers::Disabled),
///         increment
///     );
///     res1?;
///     res2?;
///     Ok(())
/// }
/// ```
///
/// **Please do note that you should never lock the output data, since this
/// will cause the paging thread to be paused. Only borrow it when it is
/// required and drop it if you have further asynchronous blocking code.**
#[cfg(feature = "async_std_lib")]
pub async fn async_std_updating(
    mutex: Lines,
    ln: LineNumbers,
) -> Result<(), AlternateScreenPagingError> {
    async_std::task::spawn(async move { run(&mutex, ln) }).await
}

/// Private function that contains the implementation for the async display.
fn run(mutex: &Lines, ln: LineNumbers) -> Result<(), AlternateScreenPagingError> {
    utils::alternate_screen_paging(ln, &mutex, |m| m.lock().unwrap())
}
