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
mod utils;
use std::sync::{Arc, Mutex};

pub use error::*;
pub use utils::LineNumbers;

/// An alias to `Arc<Mutex<Pager>>`. This allows all configuration to be updated while
/// the pager is running. Use [`Pager.finish`] for initializing it
pub type PagerMutex = Arc<Mutex<Pager>>;

/// / A struct containing basic configurations for the pager. This is used by
/// all initializing functions
///
/// ## Example
/// With any async runtime
///```
/// let pager = minus::Pager::new().set_text("Hello").set_prompt("Example").finish();
///```
///
/// For static output
///```
/// let pager = minus::Pager::new().set_text("Hello").set_prompt("Example");
///```
///
#[derive(Clone)]
pub struct Pager {
    /// The output that is displayed
    pub lines: String,
    /// Configuration for line numbers. See [`LineNumbers`]
    pub line_numbers: LineNumbers,
    pub prompt: String,
    /// The upper mark of scrolling. It is kept private to prevent end-applications
    /// cannot manipulate this
    upper_mark: usize,
}

impl Pager {
    /// Initialize a new pager configuration
    #[must_use]
    pub fn new() -> Pager {
        Pager {
            lines: String::new(),
            line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            prompt: "minus".to_string(),
        }
    }
    /// Set the output text to this `t`
    pub fn set_text(mut self, t: impl Into<String>) -> Self {
        self.lines = t.into();
        self
    }
    /// Set line number to this setting
    #[must_use]
    pub fn set_line_numbers(mut self, l: LineNumbers) -> Self {
        self.line_numbers = l;
        self
    }
    /// Set the prompt to `t`
    pub fn set_prompt(mut self, t: impl Into<String>) -> Self {
        self.prompt = t.into();
        self
    }
    /// Return a [`PagerMutex`] from this [`Pager`]. This is gated on `tokio_lib` or
    /// `async_std_lib` feature
    #[must_use]
    #[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
    pub fn finish(self) -> PagerMutex {
        Arc::new(Mutex::new(self))
    }
}

impl std::default::Default for Pager {
    fn default() -> Self {
        Pager::new()
    }
}

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
mod rt_wrappers;
#[cfg(feature = "static_output")]
mod static_pager;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
pub use rt_wrappers::*;

#[cfg(feature = "static_output")]
pub use static_pager::page_all;
