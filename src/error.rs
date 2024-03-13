//! Provides error types that are used in various places
//!
//! Some types provided are just present there to avoid leaking
//! upstream error types

use crate::minus_core::commands::Command;
use std::io;

/// An operation on the terminal failed, for example resizing it.
///
/// You can get more information about this error by calling
/// [`source`](std::error::Error::source) on it.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
#[allow(clippy::module_name_repetitions)]
pub struct TermError(
    // This member is private to avoid leaking the crossterm error type up the
    // dependency chain.
    #[from] io::Error,
);

/// There was an error while compiling the regex
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
#[allow(clippy::module_name_repetitions)]
#[cfg(feature = "search")]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
pub struct RegexError(
    // This member is private to avoid leaking the regex error type up the
    // dependency chain.
    #[from] regex::Error,
);

/// Errors that can occur during setup.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::module_name_repetitions)]
pub enum SetupError {
    #[error("The standard output is not a valid terminal")]
    InvalidTerminal,

    #[error("Failed to switch to alternate screen")]
    AlternateScreen(TermError),

    #[error("Failed to enable raw mode")]
    RawMode(TermError),

    #[error("Failed to hide the cursor")]
    HideCursor(TermError),

    #[error("Failed to enable mouse capture")]
    EnableMouseCapture(TermError),

    #[error("Couldn't determine the terminal size")]
    TerminalSize(TermError),
}

/// Errors that can occur during clean up.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::module_name_repetitions)]
pub enum CleanupError {
    #[error("Failed to disable mouse capture")]
    DisableMouseCapture(TermError),

    #[error("Failed to show the cursor")]
    ShowCursor(TermError),

    #[error("Failed to disable raw mode")]
    DisableRawMode(TermError),

    #[error("Failed to switch back to main screen")]
    LeaveAlternateScreen(TermError),
}

/// Errors that can happen during runtime.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::module_name_repetitions)]
pub enum MinusError {
    #[error("Failed to initialize the terminal")]
    Setup(#[from] SetupError),

    #[error("Failed to clean up the terminal")]
    Cleanup(#[from] CleanupError),

    #[error("Failed to draw the new data")]
    Draw(#[from] std::io::Error),

    #[error("Failed to handle terminal event")]
    HandleEvent(TermError),

    #[error("Failed to do an operation on the cursor")]
    Cursor(#[from] TermError),

    #[error("Failed to send formatted data to the pager")]
    FmtWriteError(#[from] std::fmt::Error),

    #[error("Failed to send data to the receiver")]
    Communication(#[from] crossbeam_channel::SendError<Command>),

    #[error("Failed to convert between some primitives")]
    Conversion,

    #[error(transparent)]
    #[cfg(feature = "search")]
    #[cfg_attr(docsrs, doc(cfg(feature = "search")))]
    SearchExpError(#[from] RegexError),

    #[cfg(feature = "tokio")]
    #[error(transparent)]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio")))]
    JoinError(#[from] tokio::task::JoinError),
}

// Just for  convenience helper which is useful in many places
#[cfg(feature = "search")]
impl From<regex::Error> for MinusError {
    fn from(e: regex::Error) -> Self {
        Self::SearchExpError(RegexError::from(e))
    }
}
