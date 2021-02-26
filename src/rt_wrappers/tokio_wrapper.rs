use super::*;

/// Run the pager inside a [`tokio task`](tokio::task).
///
/// This function is only available when `tokio_lib` feature is enabled.
/// It takes a [`PagerMutex`] and updates the page with new information when `PagerMutex`
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

/// use std::fmt::Write;
/// use std::time::Duration;

/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///    let output = minus::Pager::new().finish();
///
///    let increment = async {
///         for i in 0..=30_u32 {
///             let mut output = output.lock().await;
///             writeln!(output.lines, "{}", i)?;
///             drop(output);
///             sleep(Duration::from_millis(100)).await;
///          }
///          Result::<_, std::fmt::Error>::Ok(())
///      };
///
///    let (res1, res2) = join!(minus::tokio_updating(output.clone()), increment);
///    res1?;
///    res2?;
///    Ok(())
/// }
/// ```
///
/// **Please do note that you should never lock the output data, since this
/// will cause the paging thread to be paused. Only borrow it when it is
/// required and drop it if you have further asynchronous blocking code.**
#[cfg(feature = "tokio_lib")]
pub async fn tokio_updating(pager: Arc<PagerMutex>) -> Result<(), AlternateScreenPagingError> {
    tokio::task::spawn(run(pager)).await?
}
