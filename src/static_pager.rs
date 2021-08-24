//! Static information output, see [`page_all`].
use crate::{init, utils};

use crate::error::AlternateScreenPagingError;
use crate::Pager;
use crossterm::tty::IsTty;
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
/// Once called, the `Pager` passed to this function can never be changed. If you
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
///     let mut output = minus::Pager::new().unwrap();
///
///     for i in 0..=30 {
///         output.push_str(format!("{}\n", i));
///     }
///
///     minus::page_all(output)?;
///     Ok(())
/// }
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "static_output")))]
pub fn page_all(mut p: Pager) -> Result<(), PageAllError> {
    // Get stdout
    let mut stdout = io::stdout();
    let line_count = p.num_lines();

    // If stdout is not a tty, print all the output without paging and exit
    {
        if !stdout.is_tty() {
            utils::write_lines(&mut stdout, &mut p)?;
            stdout.flush()?;
            return Ok(());
        }
    }

    {
        // If the number of lines in the output is less than the number of rows
        // or run_no_overflow is true
        // display everything and quit
        if p.run_no_overflow && p.rows > line_count {
            let mut out = stdout.lock();
            utils::write_lines(&mut out, &mut p)?;
            out.flush()?;
        } else {
            init::static_paging(p)?;
        }

        Ok(())
    }
}
