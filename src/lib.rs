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

#[cfg(all(feature = "tokio_lib", feature = "async_std_lib"))]
compile_error!("Only tokio, or async_std_lib can be enabled at a time");

mod error;
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
mod rt_wrappers;
mod search;
#[cfg(feature = "static_output")]
mod static_pager;
mod utils;

#[cfg(feature = "static_output")]
pub use static_pager::page_all;

#[cfg(feature = "async_std_lib")]
pub use {async_std::sync::Mutex, rt_wrappers::async_std_wrapper::async_std_updating};
#[cfg(feature = "tokio_lib")]
pub use {rt_wrappers::tokio_wrapper::tokio_updating, tokio::sync::Mutex};

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use std::sync::Arc;

pub use error::*;

pub use utils::LineNumbers;
mod init;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
pub type PagerMutex = Mutex<Pager>;

/// A struct containing basic configurations for the pager. This is used by
/// all initializing functions
#[derive(Clone)]
pub struct Pager {
    /// The output that is displayed
    pub lines: String,
    /// Configuration for line numbers. See [`LineNumbers`]
    line_numbers: LineNumbers,
    /// The prompt displayed at the bottom
    pub prompt: String,
    // has all the data been sent to the pager
    pub data_finished: bool,
    /// The behaviour to do when user quits the program using `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    exit_strategy: ExitStrategy,
    /// The upper mark of scrolling. It is kept private to prevent end-applications
    /// from mutating this
    upper_mark: usize,
    /// Do we want to actually page if we dont have more than a screen of output
    page_if_havent_overflowed: bool,
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
    search_lines: String,
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
            lines: String::new(),
            line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            prompt: "minus".to_string(),
            exit_strategy: ExitStrategy::ProcessQuit,
            data_finished: false,
            page_if_havent_overflowed: true,
            searchable: true,
            #[cfg(feature = "search")]
            search_term: String::new(),
            #[cfg(feature = "search")]
            search_lines: String::new(),
        }
    }

    /// Set the output text to this `t`
    /// Example
    /// ```
    /// let pager = minus::Pager::new().set_text("This is a line");
    /// ```
    pub fn set_text(mut self, t: impl Into<String>) -> Self {
        self.lines = t.into();
        self
    }
    /// Set line number to this setting
    ///
    /// Example
    /// ```
    /// use minus::{Pager, LineNumbers};
    ///
    /// let pager = Pager::new().set_line_numbers(LineNumbers::Enabled);
    /// ```
    #[must_use]
    pub fn set_line_numbers(mut self, l: LineNumbers) -> Self {
        self.line_numbers = l;
        self
    }
    /// Set the prompt displayed at the prompt to `t`
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new().set_prompt("my awesome program");
    /// ```
    pub fn set_prompt(mut self, t: impl Into<String>) -> Self {
        self.prompt = t.into();
        self
    }
    /// Sets whether searching is possible inside the pager. Default s set to true
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new().set_searchable(false);
    /// ```
    ///
    /// This is a candidate for deprecation. If you want to enable search, enable the
    /// `search` feature. This is because this dosen't really give any major benifits
    /// since `regex` and all related functions are already compiled
    #[must_use]
    pub fn set_searchable(mut self, s: bool) -> Self {
        self.searchable = s;
        self
    }
    /// Sets whether the pager actually blocks UI if our data is finished, and we havent overflowed the page
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new().set_page_if_havent_overflowed(false);
    /// ```
    #[must_use]
    pub fn set_page_if_havent_overflowed(mut self, p: bool) -> Self {
        self.page_if_havent_overflowed = p;
        self
    }
    /// Return a [`PagerMutex`] from this [`Pager`]. This is gated on `tokio_lib` or
    /// `async_std_lib` feature
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new().set_text("This output is paged").finish();
    /// ```
    #[must_use]
    #[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
    pub fn finish(self) -> Arc<PagerMutex> {
        Arc::new(PagerMutex::new(self))
    }
    /// Set the default exit strategy.
    ///
    /// This controls how the pager will behave when the user presses `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    #[must_use]
    pub fn set_exit_strategy(mut self, strategy: ExitStrategy) -> Self {
        self.exit_strategy = strategy;
        self
    }

    /// Returns the appropriate text for displaying.
    ///
    /// Nrmally it will return `self.lines`
    /// In case a search, `self.search_lines` is returned
    pub(crate) fn get_lines(&self) -> String {
        #[cfg(feature = "search")]
        if self.search_term.is_empty() {
            self.lines.clone()
        } else {
            self.search_lines.clone()
        }
        #[cfg(not(feature = "search"))]
        self.lines.clone()
    }

    /// Indicate to the pager that the data has finished.
    /// This is currently only used by the async runtimes.
    pub fn data_finished(&mut self) {
        self.data_finished = true;
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
