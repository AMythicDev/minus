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

/// Errors that can occur during setup
#[derive(Debug, thiserror::Error)]
pub enum SetupError {
    #[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
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

#[derive(Debug, thiserror::Error)]
pub enum AlternateScreenPagingError {
    #[error("Failed to initialize the terminal")]
    Setup(#[from] SetupError),

    #[error("Failed to clean up the terminal")]
    Cleanup(#[from] CleanupError),

    #[error("Failed to draw the new data")]
    Draw(#[from] std::io::Error),

    #[error("Failed to handle terminal event")]
    HandleEvent(TermError),

    #[cfg(feature = "tokio_lib")]
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}