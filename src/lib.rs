//! A fast, asynchronous terminal paging library for Rust. `minus` provides high
//! level functionalities to easily write a pager for any terminal application. Due
//! to the asynchronous nature of `minus`, the pager's data can be **updated**.
//!
//! `minus` supports both [`tokio`] as well as [`async-std`] runtimes. What's more,
//! if you only want to use `minus` for serving static output, you can simply opt
//! out of these dynamic features, see the **Usage** section below.
//!
//! ## Why this crate ?
//!
//! `minus` was started by me for my work on [`pijul`]. I was unsatisfied with the
//! existing options like `pager` and `moins`.
//!
//! * `pager`:
//!     * Only provides functions to join the standard output of the current
//!       program to the standard input of external pager like `more` or `less`.
//!     * Due to this, to work within Windows, the external pagers need to be
//!       packaged along with the executable.
//!
//! * `moins`:
//!     * The output could only be defined once and for all. It is not asynchronous
//!       and does not support updating.
//!
//! [`tokio`]: https://crates.io/crates/tokio
//! [`async-std`]: https://crates.io/crates/async-std
//!
//! ## Features
//!
//! * `async_std_lib`: If your application uses [`async-std`], enable this feature.
//! * `tokio_lib`: If your application uses [`tokio`], enable this feature.
//! * `static_output`: Enable this if you only want to page static data.
//!
//! ## Examples
//!
//! See [`page_all`] for static output examples or [`async_std_updating`] and
//! [`tokio_updating`] for examples of dynamic output generation using
//! different runtimes.

#![allow(unused_imports)]
#![allow(dead_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::sync::{Arc, Mutex};

mod error;
mod utils;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
mod rt_wrappers;
#[cfg(feature = "static_output")]
mod static_pager;

/// An atomically reference counted string of all output for the pager.
pub type Lines = Arc<Mutex<String>>;

pub use error::{Error, Result, TermError};

#[cfg(feature = "tokio_lib")]
pub use rt_wrappers::tokio_updating;

#[cfg(feature = "async_std_lib")]
pub use rt_wrappers::async_std_updating;

#[cfg(feature = "static_output")]
pub use static_pager::page_all;
