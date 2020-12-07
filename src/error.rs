//! See [`Error`] and [`Result`].
use std::fmt;
use std::io;

/// Type alias for easier use of errors produced by [`minus`](crate).
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Global error type for [`minus`](crate).
///
/// You can get more informations about this error by calling
/// [`source`](std::error::Error::source) on it.
#[derive(Debug)]
pub enum Error {
    /// The error is an IO one, for example locking `stdout` failed.
    IoError(io::Error),
    /// An operation on the terminal failed, for example resizing it.
    TermError(TermError),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::TermError(e) => e.source(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(fmt, "IO-error occurred: {}", e),
            Self::TermError(e) => write!(fmt, "Operation on terminal failed: {}", e),
        }
    }
}

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

macro_rules! impl_from {
    ($from:path, $to:expr) => {
        impl From<$from> for crate::Error {
            fn from(e: $from) -> Self {
                $to(e)
            }
        }
    };
}

impl_from!(io::Error, Error::IoError);
impl_from!(TermError, Error::TermError);

impl From<crossterm::ErrorKind> for crate::Error {
    fn from(e: crossterm::ErrorKind) -> Self {
        Self::TermError(TermError(e))
    }
}
