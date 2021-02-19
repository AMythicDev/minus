/// An operation on the terminal failed, for example resizing it.
///
/// You can get more informations about this error by calling
/// [`source`](std::error::Error::source) on it.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
#[allow(clippy::module_name_repetitions)]
pub struct TermError(
    // This member is private to avoid leaking the crossterm error type up the
    // dependency chain.
    #[from] crossterm::ErrorKind,
);

/// There was an error while compiling the regex
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
#[allow(clippy::module_name_repetitions)]
#[cfg(feature = "search")]
pub struct RegexError(
    // This member is private to avoid leaking the regex error type up the
    // dependency chain.
    #[from] regex::Error,
);

/// Errors that can occur during setup
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

/// Errors that can occur during clean up
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

    #[error("Couldn't determine the terminal size")]
    TerminalSize(TermError),
}

/// Errors that can happen while running
#[derive(Debug, thiserror::Error)]
#[allow(clippy::module_name_repetitions)]
pub enum AlternateScreenPagingError {
    #[error("Failed to initialize the terminal")]
    Setup(#[from] SetupError),

    #[error("Failed to clean up the terminal")]
    Cleanup(#[from] CleanupError),

    #[error("Failed to draw the new data")]
    Draw(#[from] std::io::Error),

    #[error("Failed to handle terminal event")]
    HandleEvent(TermError),

    #[error(transparent)]
    #[cfg(feature = "search")]
    SearchExpError(#[from] RegexError),

    #[cfg(feature = "tokio_lib")]
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}
