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

//! minus is an asynchronous terminal paging library written in Rust.
//!
//! ## What is a Pager?
//! A pager is a program that lets you view and scroll through large amounts
//! of text using a keyboard in a TTY where no mouse support is available.
//!
//! Nowadays most people use a graphical terminals where mouse support is
//! present but they aren't as reliable as a pager. For example they may not support proper
//! text searching or line numbering, plus quick navigation
//! using keyboard is pretty much non-existent. Hence programs like `git`, `man` etc still use a
//! pager program to display large text outputs.
//!
//! Examples of some popular pager include `more` and its successor `less`.
//!
//! ## The problem with traditional pagers
//!
//! First, traditional pagers like `more` or `less` weren't made for integrating into other applications.
//! They were meant to be standalone binaries that are executed directly by the users.
//!
//! Applications leveraged these pagers by calling them as external programs and passing the data through
//! the standard input. This method worked for Unix and other Unix-like OSs like Linux and MacOS because
//! they already came with any of these pagers installed  But it wasn't this easy on Windows, it required
//! shipping the pager binary along with the applications. Since these programs were originally designed
//! for Unix and Unix-like OSs, distributing these binaries meant shipping an entire environment like
//! MinGW or Cygwin so that these can run properly on Windows.
//!
//! Recently, some libraries have emerged to solve this issue. They are compiled along with your
//! application and give you a single binary to distribute. The problem with them is that they
//! require you to feed the entire data to the pager before the pager can run, this meant that there will
//! be no output on the terminal until the entire data isn't loaded by the application and passed on to
//! the pager.
//!
//! These could cause long delays before output to the terminal if the data comes from a very large file
//! or is being downloaded from the internet.
//!
//! ## Enter minus
//! As above described, minus is an asynchronous terminal paging library for Rust. It allows not just
//! data but also configuration to be fed into itself while it is running.
//!
//! minus achieves this by leveraging Rust's amazing concurrency support and no data race guarantees
//!
//! minus can be used with any async runtime like [`tokio`], [`async_std`] or [`threads`] if
//! you prefer that.
//! If you want to display only static data, you don't even need to depend on any of the above
//! ## What is a Pager?
//!
//! A pager is a program that lets you view and scroll through large amounts of text using a keyboard
//! in a TTY where no mouse support is available.
//!
//! Nowadays most people use a graphical terminals where mouse support is present but they aren't as reliable as a pager. For example they may not support proper text searching or line numbering, plus quick navigation using keyboard is pretty much non-existent. Hence programs like `git`, `man` etc still use a pager program to display large text outputs.
//!
//! # Usage
//! Add minus as a dependency in your `Cargo.toml` file and enable features as you like.
//! * If you only want a pager to display static data, enable the `static_output` feature
//! * If you want a pager to display dynamic data and be configurable at runtime, enable the `dynamic_output`
//! feature
//! * If you want search support inside the pager, you need to enable the `search` feature
//! ```toml
//! [dependencies.minus]
//! version = "^5.0"
//! features = [
//!    # Enable features you want. For example
//!    "dynamic_output",
//!    "search"
//! ]
//! ```
//!
//! # Examples
//!
//! ## [`Threads`]:
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
//! ## [`tokio`]
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
//!     //  The ? mark unpacks any error that might have occured while the
//!     // pager is running
//!     res1.unwrap()?;
//!     res2?;
//!     Ok(())
//! }
//! ```
//!
//! ## Static output:
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
//! If there are more rows in the terminal than the number of lines in the given
//! data, `minus` will simply print the data and quit. This only works in static
//! //! paging since asynchronous paging could still receive more data that makes it
//! pass the limit.
//!
//! ## Standard actions
//!
//! Here is the list of default key/mouse actions handled by `minus`.
//! End-applications can change these bindings to better suit their needs.
//!
//! | Action            | Description                                                              |
//! |-------------------|--------------------------------------------------------------------------|
//! | Ctrl+C/q          | Quit the pager                                                           |
//! | <n>Arrow Up/k     | Scroll up by n number of line(s). If n is omitted it will be 1           |
//! | <n>Arrow Down/j   | Scroll down by n number of line(s). If n is omitted it will be 1         |
//! | Page Up           | Scroll up by entire page                                                 |
//! | Page Down         | Scroll down by entire page                                               |
//! | <n>Enter          | Clear prompt messages otherwise same as `k`                              |
//! | Space             | Scroll down by one page                                                  |
//! | Ctrl+U/u          | Scroll up by half a screen                                               |
//! | Ctrl+D/d          | Scroll down by half a screen                                             |
//! | g                 | Go to the very top of the output                                         |
//! | <n>G              | Go to the nth line of the output, if n is omitted, go to the very bottom |
//! | Mouse scroll Up   | Scroll up by 5 lines                                                     |
//! | Mouse scroll Down | Scroll down by 5 lines                                                   |
//! | Ctrl+L            | Toggle line numbers if not forced enabled/disabled                       |
//! | /                 | Start forward search                                                     |
//! | ?                 | Start backward search                                                    |
//! | Esc               | Cancel search input                                                      |
//! | n                 | Go to the next search match                                              |
//! | p                 | Go to the next previous match                                            |
//!
//! [`tokio`]: https://docs.rs/tokio
//! [`async-std`]: https://docs.rs/async-std

// ############################
// The Wrapping Model
// ############################
// When text is given to minus, it contains lines with lie breaks called logical
// lines. But only a certain amount of this text can be displayed on a single line
// of the terminal. This line, which makes up for one single line on the terminal
// is called a screen line.
//
// When a text is given to minus, it breaks each logical line into a `Vec<String>`.
// Each element is one screen line that is perfectly wrapped to the available
// number of columns in the terminal.
// Then all of the logical lines are stored inside a wrapper container. As a result,
// you get a `Vec<Vec<String>>`
//
// In case of prompt text and message, which are allowed to occupy only a single
// line on the terminal, and hence, must contain only one logical line are
// stored in a `Vec<String>`
//
// If the terminal size is updated, we go through each logical line, join all it's
// screen lines and wrap it again to the new configuration.
// ###################################################################################

#[cfg(feature = "dynamic_output")]
mod dynamic_pager;
pub mod error;
pub mod input;
#[path = "core/mod.rs"]
mod minus_core;
mod pager;
mod state;
#[cfg(feature = "static_output")]
mod static_pager;

#[cfg(feature = "dynamic_output")]
pub use dynamic_pager::dynamic_paging;
#[cfg(feature = "static_output")]
pub use static_pager::page_all;

#[cfg(feature = "search")]
pub use minus_core::search::SearchMode;
use std::string::ToString;

pub use pager::Pager;
pub use state::PagerState;
pub use error::MinusError;

/// A convenient type for `Vec<Box<dyn FnMut() + Send + Sync + 'static>>`
pub type ExitCallbacks = Vec<Box<dyn FnMut() + Send + Sync + 'static>>;

/// Behaviour that happens when the pager is exitted
#[derive(PartialEq, Clone, Debug)]
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

/// Enum indicating whether to display the line numbers or not.
///
/// Note that displaying line numbers may be less performant than not doing it.
/// `minus` tries to do as quickly as possible but the numbers and padding
/// still have to be computed.
///
/// This implements [`Not`](std::ops::Not) to allow turning on/off line numbers
/// when they where not locked in by the binary displaying the text.
#[derive(Debug, PartialEq, Copy, Clone)]
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
    /// Returns `true` if `self` can be inverted (i.e, `!self != self`), see
    /// the documentation for the variants to know if they are invertible or
    /// not.
    #[allow(dead_code)]
    const fn is_invertible(self) -> bool {
        matches!(self, Self::Enabled | Self::Disabled)
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

/// Rewrap a single line based on the number of columns
pub(crate) fn rewrap(line: &mut Vec<String>, cols: usize) {
    *line = textwrap::wrap(&line.join(" "), cols)
        .iter()
        .map(ToString::to_string)
        .collect();
}

/// Wrap a line of string into a `Vec<String>` based on the number of columns
pub(crate) fn wrap_str(line: &str, cols: usize) -> Vec<String> {
    textwrap::wrap(line, cols)
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<String>>()
}

#[cfg(test)]
mod tests;
