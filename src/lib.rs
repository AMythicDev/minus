#![cfg_attr(docsrs, feature(doc_cfg))]
// When no feature is active this crate is unusable but contains lots of
// unused imports and dead code. To avoid useless warnings about this they
// are allowed when no feature is active.
#![cfg_attr(
    not(any(feature = "dynamic_output", feature = "static_output")),
    allow(unused_imports),
    allow(dead_code)
)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::doc_markdown)]
#![cfg_attr(doctest, doc = include_str!("../README.md"))]

//! `minus`: A library for asynchronous terminal [paging], written in Rust.
//!
//! If you want to learn about its motivation and features, please take a look into it's [README].
//!
//! # Overview
//! When getting started with minus, the two most important concepts to get familier with are:
//! * The [Pager] type: which acts as a bridge between your application and minus. It is used
//! to pass data and configure minus before and after starting the pager.
//! * Initialization functions: This includes the [dynamic_paging] and [page_all] functions which
//! take a [Pager] as argument. They are responsible for generating the initial state and starting
//! the pager.
//!
//! See the docs for the respective items to learn more on its usage.
//!
//! # Examples
//!
//! ## Threads
//!
//! ```rust,no_run
//! use minus::{dynamic_paging, MinusError, Pager};
//! use std::{
//!     fmt::Write,
//!     thread::{spawn, sleep},
//!     time::Duration
//! };
//!
//! fn main() -> Result<(), MinusError> {
//!     // Initialize the pager
//!     let mut pager = Pager::new();
//!     // Run the pager in a separate thread
//!     let pager2 = pager.clone();
//!     let pager_thread = spawn(move || dynamic_paging(pager2));
//!
//!     for i in 0..=100_u32 {
//!         writeln!(pager, "{}", i);
//!         sleep(Duration::from_millis(100));
//!     }
//!     pager_thread.join().unwrap()?;
//!     Ok(())
//! }
//! ```
//!
//! ## tokio
//!
//! ```rust,no_run
//! use minus::{dynamic_paging, MinusError, Pager};
//! use std::time::Duration;
//! use std::fmt::Write;
//! use tokio::{join, task::spawn_blocking, time::sleep};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), MinusError> {
//!     // Initialize the pager
//!     let mut pager = Pager::new();
//!     // Asynchronously send data to the pager
//!     let increment = async {
//!         let mut pager = pager.clone();
//!         for i in 0..=100_u32 {
//!             writeln!(pager, "{}", i);
//!             sleep(Duration::from_millis(100)).await;
//!         }
//!         Result::<_, MinusError>::Ok(())
//!     };
//!     // spawn_blocking(dynamic_paging(...)) creates a separate thread managed by the tokio
//!     // runtime and runs the async_paging inside it
//!     let pager = pager.clone();
//!     let (res1, res2) = join!(spawn_blocking(move || dynamic_paging(pager)), increment);
//!     // .unwrap() unwraps any error while creating the tokio task
//!     //  The ? mark unpacks any error that might have occurred while the
//!     // pager is running
//!     res1.unwrap()?;
//!     res2?;
//!     Ok(())
//! }
//! ```
//!
//! ## Static output
//! ```rust,no_run
//! use std::fmt::Write;
//! use minus::{MinusError, Pager, page_all};
//!
//! fn main() -> Result<(), MinusError> {
//!     // Initialize a default static configuration
//!     let mut output = Pager::new();
//!     // Push numbers blockingly
//!     for i in 0..=30 {
//!         writeln!(output, "{}", i)?;
//!     }
//!     // Run the pager
//!     minus::page_all(output)?;
//!     // Return Ok result
//!     Ok(())
//! }
//! ```
//!
//! **Note:**
//! In static mode, `minus` doesn't start the pager and just prints the content if the current terminal size can
//! display all lines. You can of course change this behaviour.
//!
//! ## Default keybindings
//!
//! Here is the list of default key/mouse actions handled by `minus`.
//!
//! **A `[n] key` means that you can precede the key by an integer**.
//!
//! | Action              | Description                                                                  |
//! |---------------------|------------------------------------------------------------------------------|
//! | Ctrl+C/q            | Quit the pager                                                               |
//! | \[n\] Arrow Up/k    | Scroll up by n number of line(s). If n is omitted, scroll up by 1 line       |
//! | \[n\] Arrow Down/j  | Scroll down by n number of line(s). If n is omitted, scroll down by 1 line   |
//! | Ctrl+h              | Turn off line wrapping and allow horizontal scrolling                        |
//! | \[n\] Arrow left/h  | Scroll left by n number of line(s). If n is omitted, scroll up by 1 line     |
//! | \[n\] Arrow right/l | Scroll right by n number of line(s). If n is omitted, scroll down by 1 line  |
//! | Page Up             | Scroll up by entire page                                                     |
//! | Page Down           | Scroll down by entire page                                                   |
//! | \[n\] Enter         | Scroll down by n number of line(s).                                          |
//! | Space               | Scroll down by one page                                                      |
//! | Ctrl+U/u            | Scroll up by half a screen                                                   |
//! | Ctrl+D/d            | Scroll down by half a screen                                                 |
//! | g                   | Go to the very top of the output                                             |
//! | \[n\] G             | Go to the very bottom of the output. If n is present, goes to that line      |
//! | Mouse scroll Up     | Scroll up by 5 lines                                                         |
//! | Mouse scroll Down   | Scroll down by 5 lines                                                       |
//! | Ctrl+L              | Toggle line numbers if not forced enabled/disabled                           |
//! | Ctrl+f              | Toggle [follow-mode]                                                         |
//! | /                   | Start forward search                                                         |
//! | ?                   | Start backward search                                                        |
//! | Esc                 | Cancel search input                                                          |
//! | n                   | Go to the next search match                                                  |
//! | p                   | Go to the next previous match                                                |
//!
//! End-applications are free to change these bindings to better suit their needs. See docs for
//! [Pager::set_input_classifier] function and [input] module.
//!
//! ## Key Bindings Available at Search Prompt
//!
//! | Key Bindings      | Description                                         |
//! |-------------------|-----------------------------------------------------|
//! | Esc               | Cancel the search                                   |
//! | Enter             | Confirm the search query                            |
//! | Backspace         | Remove the character before the cursor              |
//! | Delete            | Remove the character under the cursor               |
//! | Arrow Left        | Move cursor towards left                            |
//! | Arrow right       | Move cursor towards right                           |
//! | Ctrl+Arrow left   | Move cursor towards left word by word               |
//! | Ctrl+Arrow right  | Move cursor towards right word by word              |
//! | Home              | Move cursor at the beginning pf search query        |
//! | End               | Move cursor at the end pf search query              |
//!
//! Currently these cannot be changed by applications but this may be supported in the future.
//!
//! [`tokio`]: https://docs.rs/tokio
//! [`async-std`]: https://docs.rs/async-std
//! [`Threads`]: std::thread
//! [follow-mode]: struct.Pager.html#method.follow_output
//! [paging]: https://en.wikipedia.org/wiki/Terminal_pager
//! [README]: https://github.com/arijit79/minus#motivation
#[cfg(feature = "dynamic_output")]
mod dynamic_pager;
pub mod error;
pub mod input;
#[path = "core/mod.rs"]
mod minus_core;
mod pager;
pub mod screen;
#[cfg(feature = "search")]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
pub mod search;
pub mod state;
#[cfg(feature = "static_output")]
mod static_pager;

#[cfg(feature = "dynamic_output")]
pub use dynamic_pager::dynamic_paging;
#[cfg(feature = "static_output")]
pub use static_pager::page_all;

pub use minus_core::RunMode;
#[cfg(feature = "search")]
pub use search::SearchMode;

pub use error::MinusError;
pub use pager::Pager;
pub use state::PagerState;

/// A convenient type for `Vec<Box<dyn FnMut() + Send + Sync + 'static>>`
pub type ExitCallbacks = Vec<Box<dyn FnMut() + Send + Sync + 'static>>;

/// Result type returned by most minus's functions
type Result<T = (), E = MinusError> = std::result::Result<T, E>;

/// Behaviour that happens when the pager is exited
#[derive(PartialEq, Clone, Debug, Eq)]
pub enum ExitStrategy {
    /// Kill the entire application immediately.
    ///
    /// This is the preferred option if paging is the last thing you do. For example,
    /// the last thing you do in your program is reading from a file or a database and
    /// paging it concurrently
    ///
    /// **This is the default strategy.**
    ProcessQuit,
    /// Kill the pager only.
    ///
    /// This is the preferred option if you want to do more stuff after exiting the pager. For example,
    /// if you've file system locks or you want to close database connectiions after
    /// the pager has done i's job, you probably want to go for this option
    PagerQuit,
}

/// Enum indicating whether to display the line numbers or not.
///
/// Note that displaying line numbers may be less performant than not doing it.
/// `minus` tries to do as quickly as possible but the numbers and padding
/// still have to be computed.
///
/// This implements [`Not`](std::ops::Not) to allow turning on/off line numbers
/// when they where not locked in by the binary displaying the text.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum LineNumbers {
    /// Enable line numbers permanently, cannot be turned off by user.
    AlwaysOn,
    /// Line numbers should be turned on, although users can turn it off
    /// (i.e, set it to `Disabled`).
    Enabled,
    /// Line numbers should be turned off, although users can turn it on
    /// (i.e, set it to `Enabled`).
    Disabled,
    /// Disable line numbers permanently, cannot be turned on by user.
    AlwaysOff,
}

impl LineNumbers {
    const EXTRA_PADDING: usize = 5;

    /// Returns `true` if `self` can be inverted (i.e, `!self != self`), see
    /// the documentation for the variants to know if they are invertible or
    /// not.
    #[allow(dead_code)]
    const fn is_invertible(self) -> bool {
        matches!(self, Self::Enabled | Self::Disabled)
    }

    const fn is_on(self) -> bool {
        matches!(self, Self::Enabled | Self::AlwaysOn)
    }
}

impl std::ops::Not for LineNumbers {
    type Output = Self;

    fn not(self) -> Self::Output {
        use LineNumbers::{Disabled, Enabled};

        match self {
            Enabled => Disabled,
            Disabled => Enabled,
            ln => ln,
        }
    }
}

#[cfg(test)]
mod tests;
