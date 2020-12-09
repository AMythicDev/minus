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
    doc = " **Disabled**, you cannot use `minus` with [`async-std`]."
)]
//! * `tokio_lib`:
#![cfg_attr(
    feature = "tokio_lib",
    doc = " **Enabled**, you can use `minus` with [`tokio`]. See [`tokio_updating`] for an example."
)]
#![cfg_attr(
    not(feature = "tokio_lib"),
    doc = " **Disabled**, you cannot use `minus` with [`tokio`]."
)]
//! * `static_output`:
#![cfg_attr(
    feature = "static_output",
    doc = " **Enabled**, you can use `minus` for static-only output. See [`page_all`] for an example."
)]
#![cfg_attr(
    not(feature = "static_output"),
    doc = " **Disabled**, you cannot use `minus` for static-only output."
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

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use std::sync::{Arc, Mutex};

mod error;
mod utils;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
mod rt_wrappers;
#[cfg(feature = "static_output")]
mod static_pager;

/// An atomically reference counted string of all output for the pager.
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
pub type Lines = Arc<Mutex<String>>;

pub use error::{Error, Result, TermError};
pub use utils::LineNumbers;

#[cfg(feature = "tokio_lib")]
pub use rt_wrappers::tokio_updating;

#[cfg(feature = "async_std_lib")]
pub use rt_wrappers::async_std_updating;

#[cfg(feature = "static_output")]
pub use static_pager::page_all;
