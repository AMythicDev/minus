//! A fast, asynchronous terminal paging library for Rust. `minus` provides high
//! level functionalities to easily write a pager for any terminal application.
//! Due to the asynchronous nature of `minus`, the pager's data can be
//! **updated** (this needs the correct feature to be enabled).
//!
//! `minus` supports both [`tokio`] as well as [`async-std`] runtimes. What's
//! more, if you only want to use `minus` for serving static output, you can
//! simply opt out of these dynamic features, see the
//! [**Features**](crate#features) section below.
//!
//! ## Why this crate ?
//!
//! `minus` was started by me for my work on [`pijul`]. I was unsatisfied with
//! the existing options like [`pager`] and [`moins`].
//!
//! * [`pager`]:
//!     * Only provides functions to join the standard output of the current
//!       program to the standard input of external pager like `more` or `less`.
//!     * Due to this, to work within Windows, the external pagers need to be
//!       packaged along with the executable.
//!
//! * [`moins`]:
//!     * The output could only be defined once and for all. It is not asynchronous
//!       and does not support updating.
//!
//! The main goals of `minus` are to be very compact and as configurable as possible.
//! * `minus` provides a lot of configurablity to the end-application and this
//! configuration can be defined not just in compile-time but also in **runtime.** Your
//! entire configuration like the output displayed, prompt and line numbers are inside
//! a `Arc<Mutex>`, which means at any time you can lock the configuration, change
//! something, and voila minus will automatically update the screen
//!
//! * When using `minus`, you select what features you need and **nothing else**. See
//! [Features](crate#features) below
//!
//! [`tokio`]: https://crates.io/crates/tokio
//! [`async-std`]: https://crates.io/crates/async-std
//! [`pager`]: https://crates.io/crates/pager
//! [`moins`]: https://crates.io/crates/moins
//! [`pijul`]: https://pijul.org/
//!
//! ## Features
//!
//! * `async_std_lib`: Use this if you use [`async_std`] runtime in your
//! application
//! * `tokio_lib`:Use this if you are using [`tokio`] runtime for your application
//! * `static_output`: Use this if you only want to use `minus` for displaying static
//! output
//! * `search`: If you want searching capablities inside the feature

// When no feature is active this crate is unusable but contains lots of
// unused imports and dead code. To avoid useless warnings about this they
// are allowed when no feature is active.
#![cfg_attr(
    not(any(
        feature = "tokio_lib",
        feature = "async_std_lib",
        feature = "static_output"
    )),
    allow(unused_imports),
    allow(dead_code)
)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]

mod error;
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
// mod rt_wrappers;
mod search;
#[cfg(feature = "static_output")]
mod static_pager;
// mod utils;
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
// pub use rt_wrappers::*;
#[cfg(feature = "static_output")]
pub use static_pager::page_all;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use async_mutex::Mutex;
pub use error::*;
use std::cell::UnsafeCell;
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use std::sync::Arc;
use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};
// pub use utils::LineNumbers;
// mod init;

/// A struct containing basic configurations for the pager. This is used by
/// all initializing functions
///
/// ## Example
/// You can use any async runtime, but we are taking the example of [`tokio`]
///```rust,no_run
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     use minus::{Pager, LineNumbers, tokio_updating};
///     let pager = Pager::new()
///                        .set_line_numbers(LineNumbers::AlwaysOn)
///                        .set_prompt("A complex configuration")
///                        .finish();
///
///     // Normally, you would use `futures::join` to join the pager and the text
///     // updating function. We are doing this here to make the example simple
///     tokio_updating(pager).await?;
///     Ok(())
/// }
///```
///
/// For static output
///```rust,no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///      let pager = minus::Pager::new().set_text("Hello").set_prompt("Example");
///      minus::page_all(pager)?;
///      Ok(())
/// }
///```
///
#[derive(Clone)]
pub struct Pager {
    /// The output that is displayed
    lines: Vec<String>,
    /// Configuration for line numbers. See [`LineNumbers`]
    // line_numbers: LineNumbers,
    /// The prompt displayed at the bottom
    pub prompt: String,
    /// The behaviour to do when user quits the program using `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    exit_strategy: ExitStrategy,
    /// The upper mark of scrolling. It is kept private to prevent end-applications
    /// from mutating this
    upper_mark: usize,
    /// Tells whether the searching is possible inside the pager
    ///
    /// This is a candidate for deprecation. If you want to enable search, enable the
    /// `search` feature. This is because this dosen't really give any major benifits
    /// since `regex` and all related functions are already compiled
    searchable: bool,
    /// Stores the most recent search term
    #[cfg(feature = "search")]
    search_term: String,
    /// A temporary space to store modifications to the lines string
    #[cfg(feature = "search")]
    search_lines: Vec<String>,
    rows: usize,
    cols: usize,
}

impl Pager {
    /// Initialize a new pager configuration
    ///
    /// Example
    /// ```
    /// let pager = minus::Pager::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Pager {
            lines: Vec::new(),
            // line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            prompt: "minus".to_string(),
            exit_strategy: ExitStrategy::ProcessQuit,
            searchable: true,
            #[cfg(feature = "search")]
            search_term: String::new(),
            #[cfg(feature = "search")]
            search_lines: String::new(),
            cols: 0,
            rows: 0,
        }
    }

    /// Set the output text to this `t`
    ///
    /// Note that unlike [`push_str`], this replaces the original text.
    /// If you want to append text, use the [`push_str`] function
    ///
    /// Example
    /// ```
    /// let mut pager = minus::Pager::new();
    /// pager.set_text("This is a line");
    /// ```
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.lines = split_at_width(&text.into(), self.cols);
    }
    /// Set line number to this setting
    ///
    /// Example
    /// ```
    /// use minus::{Pager, LineNumbers};
    ///
    /// let pager = Pager::new().set_line_numbers(LineNumbers::Enabled);
    /// ```
    // #[must_use]
    // pub fn set_line_numbers(mut self, l: LineNumbers) -> Self {
    //     self.line_numbers = l;
    //     self
    // }
    /// Set the prompt displayed at the prompt to `t`
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new();
    /// pager.set_prompt("my awesome program");
    /// ```
    pub fn set_prompt(&mut self, t: impl Into<String>) {
        self.prompt = t.into();
    }
    /// Sets whether searching is possible inside the pager. Default s set to true
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new();
    /// pager.set_searchable(false);
    /// ```
    ///
    /// This is a candidate for deprecation. If you want to enable search, enable the
    /// `search` feature. This is because this dosen't really give any major benifits
    /// since `regex` and all related functions are already compiled
    pub fn set_searchable(&mut self, s: bool) {
        self.searchable = s;
    }
    /// Return a [`PagerMutex`] from this [`Pager`]. This is gated on `tokio_lib` or
    /// `async_std_lib` feature
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new()
    /// pager.set_text("This output is paged").finish();
    /// ```
    #[must_use]
    #[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
    pub fn finish(self) -> Arc<Mutex<Pager>> {
        Arc::new(Mutex::new(self))
    }
    /// Set the default exit strategy.
    ///
    /// This controls how the pager will behave when the user presses `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    pub fn set_exit_strategy(&mut self, strategy: ExitStrategy) {
        self.exit_strategy = strategy;
    }

    /// Returns the appropriate text for displaying.
    ///
    /// Nrmally it will return `self.lines`
    /// In case of a search, `self.search_lines` is returned
    pub(crate) fn get_lines(&self) -> String {
        #[cfg(feature = "search")]
        if self.search_term.is_empty() {
            self.lines.join("\n")
        } else {
            self.search_lines.join("\n")
        }
        #[cfg(not(feature = "search"))]
        self.lines.join("\n")
    }
    pub fn push_str(&mut self, text: impl Into<String>) {
        self.lines
            .append(&mut split_at_width(&text.into(), self.cols));
    }
}

impl std::default::Default for Pager {
    fn default() -> Self {
        Pager::new()
    }
}

/// Behaviour that happens when the pager is exitted
#[derive(PartialEq, Clone)]
pub enum ExitStrategy {
    /// Kill the entire application immediately.
    ///
    /// This is the prefered option if paging is the last thing you do. For example,
    /// the last thing you do in your program is reading from a file or a database and
    /// paging it concurrently
    ///
    /// **This is the default strategy.**
    ProcessQuit,
    /// Kill the pager only.
    ///
    /// This is the prefered option if you want to do more stuff after exiting the pager. For example,
    /// if you've file system locks or you want to close database connectiions after
    /// the pager has done i's job, you probably want to go for this option
    PagerQuit,
}

pub(crate) fn split_at_width(text: &impl ToString, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();

    for l in text.to_string().lines() {
        lines.append(&mut split_line_at_width(l.to_string(), cols));
    }
    lines
}

fn split_line_at_width(mut line: String, cols: usize) -> Vec<String> {
    // Calculate on how many lines, the line needds to be broken
    let breaks = line.len() / cols;
    let mut lines = Vec::with_capacity(breaks.saturating_add(1));
    for _ in 1..breaks {
        let (line_1, line_2) = line.split_at(cols);
        lines.push(line_1.to_owned());
        line = line_2.to_string();
    }
    lines.push(line);

    lines
}

#[cfg(test)]
mod tests {
    use super::{split_line_at_width, Pager};
    const COLS: usize = 80;

    #[test]
    fn test_split_line_at_width_long() {
        let mut test_str = String::new();

        for _ in 0..=200 {
            test_str.push('#')
        }
        let (line_1, line_2) = test_str.split_at(COLS);
        let expected = vec![line_1.to_string(), line_2.to_string()];

        assert_eq!(expected, split_line_at_width(test_str, COLS));
    }

    #[test]
    fn test_split_line_at_width_short() {
        let mut test_str = String::new();

        for _ in 0..=50 {
            test_str.push('#')
        }

        assert_eq!(vec![test_str.clone()], split_line_at_width(test_str, COLS));
    }

    #[test]
    fn test_set_text() {
        let mut test_str = String::new();
        for _ in 0..=200 {
            test_str.push('#')
        }
        let (line_1, line_2) = test_str.split_at(COLS);
        let expected = vec![line_1.to_string(), line_2.to_string()];

        let mut pager = Pager::new();
        pager.cols = COLS;
        pager.set_text(test_str);
        assert_eq!(expected, pager.lines);
    }

    #[test]
    fn test_push_str() {
        let mut initial_str = String::new();
        for _ in 0..=50 {
            initial_str.push('#');
        }
        initial_str.push('\n');

        let mut test_str = String::new();
        for _ in 0..=200 {
            test_str.push('#')
        }

        let mut pager = Pager::new();
        pager.cols = COLS;
        pager.set_text(&initial_str);
        pager.push_str(&test_str);

        let (line_1, line_2) = test_str.split_at(COLS);
        // Remove the last \n
        initial_str.pop();
        let expected = vec![initial_str, line_1.to_string(), line_2.to_string()];

        assert_eq!(expected, pager.lines);
    }
}
