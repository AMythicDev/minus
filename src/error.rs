//! See [`Error`] and [`Result`].
use std::{fmt, io};
use thiserror::Error;

/// Different error types that could be encountered in runtime
#[derive(Debug, Error)]
pub enum Error {
    /// Input-output errors like locking the standard output
    #[error("There was an input-output error")]
    Io(io::Error),
    /// Standard output is not a terminal
    #[error("The standard output is not a valid terminal")]
    InvalidTerm,
    /// Operation on terminal failed
    #[error("Failure in terminal operation")]
    Term(TermError),
    /// The tokio task has panicked or cancelled
    ///
    /// Gated on the `tokio_lib` feature
    #[cfg(feature = "tokio_lib")]
    #[error("The Tokio task had panicked or was cancelled")]
    Join(tokio::task::JoinError),
}

/// Type alias for easier use of errors produced by [`minus`](crate).
pub type Result<T = (), E = anyhow::Error> = anyhow::Result<T, E>;

/// An operation on the terminal failed, for example resizing it.
///
/// You can get more informations about this error by calling
/// [`source`](std::error::Error::source) on it.
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct TermError(
    // This member is private to avoid leaking the crossterm error type up the
    // dependency chain.
    crossterm::ErrorKind,
);

impl std::error::Error for TermError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl fmt::Display for TermError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

impl From<crossterm::ErrorKind> for crate::Error {
    fn from(e: crossterm::ErrorKind) -> Self {
        Self::Term(TermError(e))
    }
}
