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
mod init;
pub mod input;
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
mod rt_wrappers;
#[cfg(feature = "search")]
mod search;
#[cfg(feature = "static_output")]
mod static_pager;
mod utils;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use async_mutex::Mutex;
use crossterm::{terminal, tty::IsTty};
pub use error::*;
pub use input::{DefaultInputHandler, InputHandler};
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
pub use rt_wrappers::*;
#[cfg(feature = "static_output")]
pub use static_pager::page_all;
use std::io::{self, stdout};
use std::{iter::Flatten, string::ToString, vec::IntoIter};
pub use utils::LineNumbers;
#[cfg(feature = "search")]
pub use utils::SearchMode;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
pub type PagerMutex = std::sync::Arc<Mutex<Pager>>;
pub type ExitCallbacks = Vec<Box<dyn FnMut() + Send + Sync + 'static>>;

/// A struct containing basic configurations for the pager. This is used by
/// all initializing functions
///
/// ## Example
/// You can use any async runtime, but we are taking the example of [`tokio`]
///```rust,no_run
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     use minus::{Pager, LineNumbers, tokio_updating};
///     let mut pager = Pager::new().unwrap();
///     pager.set_line_numbers(LineNumbers::AlwaysOn);
///     pager.set_prompt("A complex configuration");
///
///     // Normally, you would use `futures::join` to join the pager and the text
///     // updating function. We are doing this here to make the example simple
///     tokio_updating(pager.finish()).await?;
///     Ok(())
/// }
///```
///
/// For static output
///```rust,no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///      let mut pager = minus::Pager::new().unwrap();
///      pager.set_text("Hello");
///      pager.set_prompt("Example");
///      minus::page_all(pager)?;
///      Ok(())
/// }
///```
///
pub struct Pager {
    /// The output that is displayed
    /// Represented by a vector of lines where each line is a vector of strings
    /// split up on the basis of terminal width
    wrap_lines: Vec<Vec<String>>,
    /// Configuration for line numbers. See [`LineNumbers`]
    pub(crate) line_numbers: LineNumbers,
    /// The prompt displayed at the bottom
    prompt: String,
    /// Text which may have come through writeln that is unwraped
    lines: String,
    /// The input handler to be called when a input is found
    input_handler: Box<dyn input::InputHandler + Sync + Send>,
    /// Functions to run when the pager quits
    exit_callbacks: Vec<Box<dyn FnMut() + Send + Sync + 'static>>,
    /// The behaviour to do when user quits the program using `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    exit_strategy: ExitStrategy,
    /// Whether the coming data is ended
    ///
    /// Applications should strictly call [Pager::end_data_stream()] once their stream
    /// of data to the pager is ended.
    end_stream: bool,
    /// The upper mark of scrolling. It is kept private to prevent end-applications
    /// from mutating this
    pub(crate) upper_mark: usize,
    /// Do we want to page if there;s no overflow
    pub(crate) run_no_overflow: bool,
    /// Stores the most recent search term
    #[cfg(feature = "search")]
    search_term: Option<regex::Regex>,
    // Direction of search
    #[cfg(feature = "search")]
    search_mode: SearchMode,
    /// Lines where searches have a match
    #[cfg(feature = "search")]
    pub(crate) search_idx: Vec<u16>,
    /// Rows of the terminal
    pub(crate) rows: usize,
    /// Columns of the terminal
    pub(crate) cols: usize,
}

impl Pager {
    /// Initialize a new pager configuration
    ///
    /// ## Errors
    /// This function will return an error if it cannot determine the terminal size
    ///
    /// # Example
    /// ```
    /// let pager = minus::Pager::new().unwrap();
    /// ```
    pub fn new() -> Result<Self, error::AlternateScreenPagingError> {
        let (rows, cols);

        if cfg!(test) {
            // In tests, set these number of columns to 80 and rows to 10
            cols = 80;
            rows = 10;
        } else if stdout().is_tty() {
            // If a proper terminal is present, get size and set it
            let size = terminal::size()?;
            cols = size.0;
            rows = size.1;
        } else {
            // For other cases beyond control
            cols = 1;
            rows = 1;
        };

        Ok(Pager {
            wrap_lines: Vec::new(),
            line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            prompt: "minus".to_string(),
            exit_strategy: ExitStrategy::ProcessQuit,
            input_handler: Box::new(input::DefaultInputHandler {}),
            exit_callbacks: Vec::new(),
            run_no_overflow: false,
            lines: String::new(),
            end_stream: false,
            #[cfg(feature = "search")]
            search_term: None,
            #[cfg(feature = "search")]
            search_mode: SearchMode::Unknown,
            #[cfg(feature = "search")]
            search_idx: Vec::new(),
            // Just to be safe in tests, keep at 1x1 size
            cols: cols as usize,
            rows: rows as usize,
        })
    }

    /// Set the output text to this `t`
    ///
    /// Note that unlike [`Pager::push_str`], this replaces the original text.
    /// If you want to append text, use the [`Pager::push_str`] function
    ///
    /// Example
    /// ```
    /// let mut pager = minus::Pager::new().unwrap();
    /// pager.set_text("This is a line");
    /// ```
    pub fn set_text(&mut self, text: impl Into<String>) {
        let text: String = text.into();
        // self.lines = WrappedLines::from(Line::from_str(&text.into(), self.cols));
        self.wrap_lines = text.lines().map(|l| wrap_str(l, self.cols)).collect();
    }

    /// Set line number to this setting
    ///
    /// Example
    /// ```
    /// use minus::{Pager, LineNumbers};
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.set_line_numbers(LineNumbers::Enabled);
    /// ```
    pub fn set_line_numbers(&mut self, l: LineNumbers) {
        self.line_numbers = l;
    }

    /// Set the prompt displayed at the prompt to `t`
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.set_prompt("my awesome program");
    /// ```
    pub fn set_prompt(&mut self, t: impl Into<String>) {
        self.prompt = t.into();
    }

    /// Return a [`PagerMutex`] from this [`Pager`]. This is gated on `tokio_lib` or
    /// `async_std_lib` feature
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.set_text("This output is paged");
    /// let _pager_mutex = pager.finish();
    /// ```
    #[must_use]
    #[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
    pub fn finish(self) -> PagerMutex {
        std::sync::Arc::new(Mutex::new(self))
    }

    /// Set the default exit strategy.
    ///
    /// This controls how the pager will behave when the user presses `q` or `Ctrl+C`.
    /// See [`ExitStrategy`] for available options
    ///
    /// ```
    /// use minus::{Pager, ExitStrategy};
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.set_exit_strategy(ExitStrategy::ProcessQuit);
    /// ```
    pub fn set_exit_strategy(&mut self, strategy: ExitStrategy) {
        self.exit_strategy = strategy;
    }

    /// Returns the appropriate text for displaying.
    ///
    /// Nrmally it will return `self.lines`
    /// In case of a search, `self.search_lines` is returned
    pub(crate) fn get_lines(&self) -> Vec<Vec<String>> {
        self.wrap_lines.clone()
    }

    /// Set whether to display pager if there's less data than
    /// available screen height
    ///
    /// By default this is set to false
    ///
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.set_run_no_overflow(true);
    /// ```
    pub fn set_run_no_overflow(&mut self, value: bool) {
        self.run_no_overflow = value;
    }

    /// Appends text to the pager output
    ///
    /// This function will automatically split the lines, if they overflow
    /// the number of terminal columns
    ///
    /// ```
    /// let mut pager = minus::Pager::new().unwrap();
    /// pager.push_str("This is some text");
    /// ```
    pub fn push_str(&mut self, text: impl Into<String>) {
        let text: String = text.into();
        text.lines()
            .for_each(|l| self.wrap_lines.push(wrap_str(l, self.cols)));
    }

    /// Tells the running pager that no more data is coming
    ///
    /// Note that after this function is called, any call to [`Pager::set_text()`] or
    /// [`Pager::push_str()`] will panic
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.set_text("Hello from minus!");
    /// pager.end_data_stream();
    /// ```
    pub fn end_data_stream(&mut self) {
        self.end_stream = true;
    }

    /// Readjust the text to new terminal size
    pub(crate) fn readjust_wraps(&mut self) {
        rewrap_lines(&mut self.wrap_lines, self.cols)
    }

    /// Returns all the text by flattening them into a single vector of strings
    pub(crate) fn get_flattened_lines(&self) -> Flatten<IntoIter<Vec<String>>> {
        self.get_lines().into_iter().flatten()
    }

    /// Returns the number of lines the [`Pager`] currently holds
    pub(crate) fn num_lines(&self) -> usize {
        self.get_flattened_lines().count()
    }

    /// Set custom input handler function
    ///
    /// See example in [`InputHandler`](input::InputHandler) on using this
    /// function
    pub fn set_input_handler(&mut self, handler: Box<dyn input::InputHandler + Send + Sync>) {
        self.input_handler = handler;
    }

    /// Run the exit callbacks
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// fn hello() {
    ///     println!("Hello");
    /// }
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.add_exit_callback(Box::new(hello));
    /// pager.exit()
    /// ```
    pub fn exit(&mut self) {
        for func in &mut self.exit_callbacks {
            func()
        }
    }

    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// fn hello() {
    ///     println!("Hello");
    /// }
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.add_exit_callback(Box::new(hello));
    /// ```
    pub fn add_exit_callback(&mut self, cb: impl FnMut() + Send + Sync + 'static) {
        self.exit_callbacks.push(Box::new(cb));
    }
}

impl std::default::Default for Pager {
    fn default() -> Self {
        Pager::new().unwrap()
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

/// Rewrap already wrapped vector of lines based on the number of columns
pub(crate) fn rewrap_lines(lines: &mut Vec<Vec<String>>, cols: usize) {
    for line in lines {
        rewrap(line, cols);
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

impl io::Write for Pager {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let string = String::from_utf8_lossy(buf);
        self.lines.push_str(&string);
        if string.ends_with('\n') {
            self.flush()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        for line in self.lines.lines() {
            self.wrap_lines.push(wrap_str(line, self.cols))
        }
        self.lines.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
