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
