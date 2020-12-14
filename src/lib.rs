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
//! [`tokio`]: https://crates.io/crates/tokio
//! [`async-std`]: https://crates.io/crates/async-std
//! [`pager`]: https://crates.io/crates/pager
//! [`moins`]: https://crates.io/crates/moins
//! [`pijul`]: https://pijul.org/
//!
//! ## Features
//!
//! * `async_std_lib`:
#![cfg_attr(
    feature = "async_std_lib",
    doc = " **Enabled**, you can use `minus` with [`async-std`]. See [`async_std_updating`] for an example."
)]
#![cfg_attr(
    not(feature = "async_std_lib"),
    doc = " **Disabled**, you cannot use `minus` with [`async-std`] because of your current configuration."
)]
//! * `tokio_lib`:
#![cfg_attr(
    feature = "tokio_lib",
    doc = " **Enabled**, you can use `minus` with [`tokio`]. See [`tokio_updating`] for an example."
)]
#![cfg_attr(
    not(feature = "tokio_lib"),
    doc = " **Disabled**, you cannot use `minus` with [`tokio`] because of your current configuration."
)]
//! * `static_output`:
#![cfg_attr(
    feature = "static_output",
    doc = " **Enabled**, you can use `minus` for static-only output. See [`page_all`] for an example."
)]
#![cfg_attr(
    not(feature = "static_output"),
    doc = " **Disabled**, you cannot use `minus` for static-only output because of your current configuration."
)]
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
/// the pager is running. Use [`Pager::new_dynamic`] and [`Pager::default_dynamic`] for
/// initializing it
pub type PagerMutex = Arc<Mutex<Pager>>;

/// / A struct containing basic configurations for the pager. This is used by
/// all initializing functions
#[derive(Clone)]
pub struct Pager {
    /// The output that is displayed
    pub lines: String,
    /// Configuration for line numbers. See [`LineNumbers`]
    pub line_numbers: LineNumbers,
    /// The upper mark of scrolling. It is kept private so that end-applications cannot
    /// manipulate this
    upper_mark: usize,
}

impl Pager {
    #[cfg(any(feature = "async_std_lib", feature = "tokio_lib"))]
    #[must_use = "This function must be used in dynamic paging"]
    /// Returns a new [`PagerMutex`] from the given text and line number configuration
    ///
    /// ## Example
    /// Works with any async runtime
    ///```
    /// use minus::{Pager, LineNumbers};
    ///
    /// let pager = Pager::new_dynamc(String::new(), LineNumbers::Disabled);
    ///```
    pub fn new_dynamic(lines: String, ln: LineNumbers) -> PagerMutex {
        Arc::new(Mutex::new(Pager {
            lines,
            line_numbers: ln,
            upper_mark: 0,
        }))
    }
    #[cfg(feature = "static_output")]
    #[must_use = "This function must be used in static paging"]
    /// Returns a new [`Pager`] from the given text and line number configuration
    pub fn new_static(lines: String, ln: LineNumbers) -> Pager {
        Pager {
            lines,
            line_numbers: ln,
            upper_mark: 0,
        }
    }
    #[cfg(feature = "static_output")]
    #[must_use = "This function must be used in static paging"]
    /// Returns a new [`Pager`] with the some defaults, like an empty string and line
    /// numbers set to be disabled. For furthur customizations, use the
    /// [`new_static`](Pager::new_static) function
    pub fn default_static() -> Pager {
        Pager::new_static(String::new(), LineNumbers::Disabled)
    }
    /// Returns a new [`PagerMutex`] with the some defaults, like an empty string
    /// and line numbers set to be disabled. For furthur customizations, use the
    /// [`new_dynamic`](Pager::new_dynamic) function
    #[cfg(any(feature = "async_std_lib", feature = "tokio_lib"))]
    #[must_use = "This function must be used in dynamic paging"]
    pub fn default_dynamic() -> PagerMutex {
        Pager::new_dynamic(String::new(), LineNumbers::Disabled)
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
