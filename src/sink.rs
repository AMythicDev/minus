//! Defines the [`OutputSink`] trait and its implementations.

use crossterm::tty::IsTty;
use std::io::Write;

/// A trait for configuring the output sink for minus.
///
/// By default, minus writes all formatted text and terminal control sequences to
/// [`std::io::stdout`]. By implementing this trait or using the provided implementations,
/// you can redirect minus's output to other sinks such as [`std::io::stderr`], `/dev/tty`
/// (via [`std::fs::File`]), or custom buffers.
///
/// # Implementations
/// minus provides implementations of [`OutputSink`] for:
/// - [`std::io::Stdout`]
/// - [`std::io::Stderr`]
/// - [`std::fs::File`]
/// - [`Vec<u8>`]
/// - [`std::io::Cursor<T>`]
/// - [`std::io::Sink`]
/// - [`Box<T>`] where `T: OutputSink + ?Sized`
pub trait OutputSink: Write + Send + Sync + 'static {
    /// Returns `true` if the sink is connected to a terminal / TTY.
    fn is_tty(&self) -> bool {
        false
    }
}

impl OutputSink for std::io::Stdout {
    fn is_tty(&self) -> bool {
        IsTty::is_tty(self)
    }
}

impl OutputSink for std::io::Stderr {
    fn is_tty(&self) -> bool {
        IsTty::is_tty(self)
    }
}

impl OutputSink for std::fs::File {
    fn is_tty(&self) -> bool {
        IsTty::is_tty(self)
    }
}

impl OutputSink for Vec<u8> {
    fn is_tty(&self) -> bool {
        false
    }
}

impl<T: AsRef<[u8]> + Send + Sync + 'static> OutputSink for std::io::Cursor<T>
where
    Self: Write,
{
    fn is_tty(&self) -> bool {
        false
    }
}

impl OutputSink for std::io::Sink {
    fn is_tty(&self) -> bool {
        false
    }
}

impl<T: OutputSink + ?Sized> OutputSink for Box<T> {
    fn is_tty(&self) -> bool {
        (**self).is_tty()
    }
}
