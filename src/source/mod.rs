//! Provides the [`DataSource`] trait and built-in sources for on-demand text loading.
//!
//! ## Motivation
//! Storing the entire text inside minus twice (once as the original text and once as the
//! formatted rows destined for the terminal) does not scale for very large inputs: the whole
//! text may simply not fit into memory. The [`DataSource`] trait solves this by letting
//! end-applications define where the text comes from and how minus loads it on demand. minus
//! keeps only a small cache of the rows currently needed for display and asks the source for
//! lines whenever it needs more.
//!
//! # Implementing a custom source
//! Any type that can answer "how many lines do you have?" and "give me line N" can be a data
//! source — a file on disk, a memory-mapped region, a database, a stream, etc. The trait is
//! object-safe, so sources are handed to the pager as `Box<dyn DataSource>`.
//!
//! minus is a pure *reader* of a data source: it never writes text into it. Sources that grow
//! over time (dynamic feeds) are fed by the application itself through its own means; minus
//! notices new data by re-checking [`DataSource::line_count`] and the tail state while the
//! pager runs.
//!
//! # Example
//! ```
//! use minus::source::{DataSource, InMemorySource};
//!
//! let source = InMemorySource::from("Hello\nWorld\n");
//! assert_eq!(source.line_count(), 2);
//! assert_eq!(source.line(0).as_deref(), Some("Hello"));
//! ```
//!
//! Sources that are fed while the pager runs can be shared across threads by wrapping them
//! in a mutex:
//! ```
//! use minus::source::{DataSource, InMemorySource};
//! use parking_lot::Mutex;
//! use std::sync::Arc;
//!
//! let source = Arc::new(Mutex::new(InMemorySource::new()));
//! // `Arc<Mutex<InMemorySource>>` implements `DataSource` itself
//! let dyn_source: Box<dyn DataSource> = Box::new(source.clone());
//!
//! source.lock().append("Hello\n");
//! assert_eq!(dyn_source.line_count(), 1);
//! assert_eq!(dyn_source.line(0).as_deref(), Some("Hello"));
//! ```

use std::{borrow::Cow, fmt, sync::Arc};

use parking_lot::Mutex;

/// Defines how minus loads text data on demand
///
/// Implementing this trait allows end-applications to plug their own text origins into minus
/// instead of handing over the entire text at once. This removes the need for minus to hold
/// the whole text in memory twice and makes it possible to page inputs that would not fit
/// into memory at all.
///
/// # Text model
/// Lines are the unit of everything inside minus: line numbers, wrapping, search and
/// selection all operate on lines. A line is text that must not contain any newline (`\n`)
/// inside it but may or may not end with one. This trait follows the same model:
/// [`DataSource::line`] returns a line without its trailing newline, exactly like
/// [`str::lines`] does (a trailing `\n` and a `\r` immediately preceding it are stripped).
///
/// # Requirements
/// The trait is object-safe and requires [`Send`], [`Sync`] and [`'static`](std::marker) so
/// that a `Box<dyn DataSource>` can be shared across the threads the pager runs on. All
/// methods can be called from any thread at any time while the pager is running, so
/// implementations backed by IO or locks should keep their critical sections small.
///
/// # Defaults
/// * [`DataSource::last_line_terminated`] defaults to `true`: the source's last line is
///   considered final unless the implementation says otherwise. minus uses this to know
///   whether incoming data merges into the last line or starts a new one.
/// * [`DataSource::is_complete`] defaults to `false`: minus keeps re-checking the source for
///   new data while the pager runs. Implementations that know no more data will ever arrive
///   (for example a fully-read file) can return `true` to skip these checks.
#[allow(clippy::module_name_repetitions)]
pub trait DataSource: Send + Sync + 'static {
    /// Number of logical lines currently available
    #[must_use]
    fn line_count(&self) -> usize;

    /// Line at 0-based `idx`, without trailing newline (mimics [`str::lines`]: strips a
    /// trailing `\n` and a `\r` immediately preceding it)
    ///
    /// Returns [`None`] if `idx` is out of bounds.
    #[must_use]
    fn line(&self, idx: usize) -> Option<Cow<'_, str>>;

    /// Whether the last line may still receive more text (mid-line append)
    #[must_use]
    fn last_line_terminated(&self) -> bool {
        true
    }

    /// Whether no more data will ever arrive (lets minus skip diff polling)
    #[must_use]
    fn is_complete(&self) -> bool {
        false
    }
}

/// An in-memory [`DataSource`] thatborrow::Cow,  stores the text exactly once
///
/// This is the default source minus uses: text fed through appends is stored here as a
/// single [`String`] with a byte-offset index over the line starts, making random access to
/// any line cheap while keeping the memory usage to the text itself.
///
/// # Example
/// ```
/// use minus::source::{DataSource, InMemorySource};
/// use std::fmt::Write;
///
/// let mut source = InMemorySource::new();
/// write!(source, "Hello {}", "World").unwrap();
/// // Appends merge into an unterminated last line
/// source.append(" and everyone");
/// assert_eq!(source.line_count(), 1);
/// assert_eq!(source.line(0).as_deref(), Some("Hello World and everyone"));
/// ```
#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Default)]
pub struct InMemorySource {
    /// The entire text stored exactly once
    text: String,
    /// Byte offset of the start of each line
    line_starts: Vec<usize>,
    /// Whether no more data will ever arrive
    complete: bool,
}

impl InMemorySource {
    /// Create an empty source
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append raw text to the end of the source
    ///
    /// If the last line of the source is unterminated (i.e, the source is not empty and does
    /// not end with a newline), the incoming text is part of that line so it merges into it.
    /// Otherwise the text starts a new line. An empty text changes nothing.
    pub fn append(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        // If the source is empty or its last line is terminated, the incoming text starts a
        // new line at the current end of the text. Otherwise it merges into the unterminated
        // last line and no line start is recorded for it.
        if self.text.is_empty() || self.text.ends_with('\n') {
            self.line_starts.push(self.text.len());
        }

        let append_start = self.text.len();
        self.text.push_str(s);

        // Every newline inside the appended text that is not the very last byte of the
        // resulting text starts a new line right after it. A newline at the very end only
        // becomes the start of a line once more text is appended after it.
        for (idx, byte) in s.bytes().enumerate() {
            if byte == b'\n' {
                let pos = append_start + idx;
                if pos + 1 < self.text.len() {
                    self.line_starts.push(pos + 1);
                }
            }
        }
    }

    /// Replace the entire text with `s`
    ///
    /// This resets everything including the completion state — replacing the content voids a
    /// previous [`Self::finish`] promise, so the source becomes incomplete again until
    /// [`Self::finish`] is called.
    pub fn replace(&mut self, s: &str) {
        self.text.clear();
        self.line_starts.clear();
        self.complete = false;
        self.append(s);
    }

    /// Mark the source as complete, i.e, no more data will ever arrive
    ///
    /// The last line stays as it is. minus uses this to skip re-checking the source for new
    /// data while the pager runs.
    pub fn finish(&mut self) {
        self.complete = true;
    }
}

impl fmt::Write for InMemorySource {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.append(s);
        Ok(())
    }
}

impl<T> From<T> for InMemorySource
where
    T: AsRef<str>,
{
    fn from(s: T) -> Self {
        let mut source = Self::new();
        source.append(s.as_ref());
        source
    }
}

/// Each element is appended as raw text in order, consistent with [`InMemorySource::append`].
/// Collecting elements that do not end with a newline merges them into one line.
impl FromIterator<String> for InMemorySource {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut source = Self::new();
        for s in iter {
            source.append(&s);
        }
        source
    }
}

impl DataSource for InMemorySource {
    fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    fn line(&self, idx: usize) -> Option<Cow<'_, str>> {
        let start = *self.line_starts.get(idx)?;
        // The line ends where the next line starts, or at the end of the text for the last
        // line.
        let end = self
            .line_starts
            .get(idx + 1)
            .copied()
            .unwrap_or(self.text.len());
        // Mimic `str::lines()`: strip the trailing '\n' of a terminated line and a '\r'
        // immediately preceding it. The `end > start` guard keeps an empty terminated line
        // from stripping into the preceding line's terminator.
        let bytes = self.text.as_bytes();
        let mut end = end;
        if end > start && bytes[end - 1] == b'\n' {
            end -= 1;
            if end > start && bytes[end - 1] == b'\r' {
                end -= 1;
            }
        }
        Some(Cow::Borrowed(&self.text[start..end]))
    }

    fn last_line_terminated(&self) -> bool {
        self.text.is_empty() || self.text.ends_with('\n')
    }

    fn is_complete(&self) -> bool {
        self.complete
    }
}

impl<S: DataSource + ?Sized> DataSource for Mutex<S> {
    fn line_count(&self) -> usize {
        self.lock().line_count()
    }

    fn line(&self, idx: usize) -> Option<Cow<'_, str>> {
        // The MutexGuard temporary cannot outlive this call, so the borrowed variant of the
        // inner source cannot escape the lock. Materialize the line into an owned Cow
        // instead.
        self.lock()
            .line(idx)
            .map(|cow| Cow::Owned(cow.into_owned()))
    }

    fn last_line_terminated(&self) -> bool {
        self.lock().last_line_terminated()
    }

    fn is_complete(&self) -> bool {
        self.lock().is_complete()
    }
}

impl<S: DataSource + ?Sized> DataSource for Arc<S> {
    fn line_count(&self) -> usize {
        self.as_ref().line_count()
    }

    fn line(&self, idx: usize) -> Option<Cow<'_, str>> {
        // The data lives as long as the Arc, so the borrowed variant can be returned
        // directly without copying.
        self.as_ref().line(idx)
    }

    fn last_line_terminated(&self) -> bool {
        self.as_ref().last_line_terminated()
    }

    fn is_complete(&self) -> bool {
        self.as_ref().is_complete()
    }
}

#[cfg(test)]
mod tests;
