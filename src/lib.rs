//! Minus is a library for creating paged output for othe terminal based
//! applications. It is threaded as well as asynchronous, which means your
//! applications can give dynamic information. It is also cross-platform
//! which means your applications are assured to be 100% compatible with all OSs
//!
//! ## Why use minus
//! * Pager runs in a separate thread which is asynchronous
//! * Works with both tokio and async_std, these are individual features you can
//! enable. So you are confirmed that you don't put bloat in your software
//! * Completely cross-platform
//!
//! ## Features
//! * `async_std_lib`:- If your application uses async-std, enable this feature
//! * `tokio_lib`:- If your application uses tokio, enable this feature
//! * `static_output`: Enable this if you only want to page static data
//!
//! ## Examples
//! See [page_all] for static output examples or [async_std_updating] and
//! [tokio_updating] for examples of dynamic output generation using different runtimes

#![allow(unused_imports)]
#![allow(dead_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]

mod utils;
use std::sync::{Arc, Mutex};
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
mod rt_wrappers;
#[cfg(feature = "static_output")]
mod static_pager;

/// An atomically reference counted string of all output for the pager
pub type Lines = Arc<Mutex<String>>;

#[cfg(feature = "tokio_lib")]
pub use rt_wrappers::tokio_updating;

#[cfg(feature = "async_std_lib")]
pub use rt_wrappers::async_std_updating;

#[cfg(feature = "static_output")]
pub use static_pager::page_all;
