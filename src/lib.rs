#![cfg_attr(docsrs, feature(doc_cfg))]

//! `minus` is a terminal paging library for Rust. It provides high
//! level functions to easily embed a pager for any terminal application without
//! requiring you to think about the nitty-gritty of building a pager.
//!
//! `minus` can be used in asynchronous mode or in a blocking fashion.
//!
//! In asynchronous mode, the pager's data as well as it's
//! configuration can be **updated** at any time. `minus` supports both
//! [`tokio`] as well as [`async-std`] runtimes. These runtimes are gated
//! on specific [features](./index.html#features).
//!
//! In blocking mode, the pager stops any other task from being executed. This
//! is good if you want to show some static information but it does not allow
//! you to change the configuration of the pager at runtime.
//!
//! # Features
//! * `async_std_lib`: Use this if you use [`async_std`] runtime in your
//! application
//! * `tokio_lib`:Use this if you are using [`tokio`] runtime for your application
//! * `static_output`: Use this if you only want to use `minus` for displaying static
//! output
//! * `search`: If you want searching capablities inside the feature
//!
//! **Note:** You must select a either a runtime feature or `static_output`. If you do not
//! select any features when adding `minus` to `Cargo.toml`, it will not have any functions
//! to start a pager.
//!
//! # Examples
//! Print numbers 1 through 100 with 100ms delay in asynchronous mode
//!
//! You can use any async runtime, but we are taking the example of [`tokio`]
//!```rust,no_run
//! use futures::join;
//! use minus::{Pager, tokio_updating};
//! use std::{fmt::Write, time::Duration};
//! use tokio::time::sleep;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut pager = Pager::new()?;
//!     pager.set_prompt("An asynchronous example");
//!     let pager_mutex = pager.finish();
//!
//!     let updater = async {
//!         for i in 1..=100u8 {
//!             let mut guard = pager_mutex.lock().await;
//!             writeln!(guard, "{}", i)?;
//!             // Remember to drop the guard before any await or blocking operation
//!             drop(guard);
//!             sleep(Duration::from_millis(100)).await;
//!         }
//!         let mut guard = pager_mutex.lock().await;
//!         guard.end_data_stream();
//!         Result::<_, std::fmt::Error>::Ok(())
//!     };
//!
//!     let (res1, res2) = join!(tokio_updating(pager_mutex.clone()), updater);
//!     res1?;
//!     res2?;
//!     Ok(())
//! }
//!```
//!
//! Print 1 through 100 in a blocking fashion (static output)
//!```rust,no_run
//! use std::fmt::Write;
//! use minus::page_all;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!      let mut pager = minus::Pager::new().unwrap();
//!      for i in 1..=100 {
//!         writeln!(pager, "{}", i)?;
//!      }
//!      pager.set_prompt("Example");
//!      minus::page_all(pager)?;
//!      Ok(())
//! }
//!```
//!
//! [`tokio`]: https://crates.io/crates/tokio
//! [`async-std`]: https://crates.io/crates/async-std
//! [`pager`]: https://crates.io/crates/pager
//! [`moins`]: https://crates.io/crates/moins
//! [`pijul`]: https://pijul.org/

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

pub mod error;
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
use error::AlternateScreenPagingError;
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
pub use rt_wrappers::*;
#[cfg(feature = "search")]
pub use search::SearchMode;
#[cfg(feature = "static_output")]
pub use static_pager::page_all;
use std::string::ToString;
use std::{fmt, io::stdout};
pub use utils::LineNumbers;

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
#[cfg_attr(
    docsrs,
    doc(cfg(any(feature = "tokio_lib", feature = "async_std_lib")))
)]
/// A convenient type for `std::sync::Arc<async_mutex::Mutex<Pager>>`
pub type PagerMutex = std::sync::Arc<Mutex<Pager>>;
/// A convenient type for `Vec<Box<dyn FnMut() + Send + Sync + 'static>>`
pub type ExitCallbacks = Vec<Box<dyn FnMut() + Send + Sync + 'static>>;

// The Wrapping Model
//
// minus heavily uses the wrapping model. This is key to understand how minus
// internally.
//
// When a text is given to minus for displaying, it internally takes each
// logical line of it and breaks it into a `Vec<String>`. To hold all of
// those lines, it stores them inside another Vec container. This makes it the
// `Vec<Vec<String>>` struct.
//
// Each element in the 1st `Vec` is a logical line. While each String in the 2nd
// `Vec` is a line wrapped to the available terminal width.
//
// In case of prompt text and message, which are allowed to occupy only a single
//  row in the terminal, and hence, must contain only 1 line are contained in a
// `Vec<String>`
//
// If the terminal is resized, we update the rows and columns and rewrap the
// text

/// A struct containing all configurations for the pager.
///
/// This is used by all initializing functions
pub struct Pager {
    // The text the pager has been told to be displayed
    lines: String,
    // The output, flattened and formatted into the lines that should be displayed
    formatted_lines: Vec<String>,
    // Configuration for line numbers. See [`LineNumbers`]
    pub(crate) line_numbers: LineNumbers,
    // The prompt displayed at the bottom wrapped to available terminal width
    prompt: Vec<String>,
    // The input classifier to be called when a input is found
    input_classifier: Box<dyn input::InputClassifier + Sync + Send>,
    // Functions to run when the pager quits
    exit_callbacks: Vec<Box<dyn FnMut() + Send + Sync + 'static>>,
    // The behaviour to do when user quits the program using `q` or `Ctrl+C`
    // See [`ExitStrategy`] for available options
    exit_strategy: ExitStrategy,
    // Whether the coming data is ended
    //
    // Applications should strictly call [Pager::end_data_stream()] once their stream
    // of data to the pager is ended.
    end_stream: bool,
    // Any warning or error to display to the user at the prompt
    // The first element contains the actual message, while the second element tells
    // whether the message has changed since the last display.
    message: (Option<Vec<String>>, bool),
    // The upper mark of scrolling. It is kept private to prevent end-applications
    // from mutating this
    pub(crate) upper_mark: usize,
    // Do we want to page if there's no overflow
    pub(crate) run_no_overflow: bool,
    // Stores the most recent search term
    #[cfg(feature = "search")]
    search_term: Option<regex::Regex>,
    // Direction of search
    #[cfg(feature = "search")]
    search_mode: SearchMode,
    // Lines where searches have a match
    #[cfg(feature = "search")]
    pub(crate) search_idx: Vec<usize>,
    // Rows of the terminal
    pub(crate) rows: usize,
    // Columns of the terminal
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
    pub fn new() -> Result<Self, error::TermError> {
        let (rows, cols);

        if cfg!(test) {
            // In tests, set these number of columns to 80 and rows to 10
            cols = 80;
            rows = 10;
        } else if stdout().is_tty() {
            // If a proper terminal is present, get size and set it
            let size = terminal::size()?;
            cols = size.0 as usize;
            rows = size.1 as usize;
        } else {
            // For other cases beyond control
            cols = 1;
            rows = 1;
        };

        Ok(Pager {
            lines: String::new(),
            formatted_lines: Vec::new(),
            line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            prompt: wrap_str("minus", cols),
            exit_strategy: ExitStrategy::ProcessQuit,
            input_classifier: Box::new(input::DefaultInputClassifier {}),
            exit_callbacks: Vec::new(),
            run_no_overflow: true,
            message: (None, false),
            end_stream: false,
            #[cfg(feature = "search")]
            search_term: None,
            #[cfg(feature = "search")]
            search_mode: SearchMode::Unknown,
            #[cfg(feature = "search")]
            search_idx: Vec::new(),
            // Just to be safe in tests, keep at 1x1 size
            cols,
            rows,
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
        self.lines = text.into();
        self.format_lines();
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
        self.format_lines();
    }

    /// Display a temporary message at the prompt area
    ///
    /// # Panics
    /// This function panics if the given text contains newline characters.
    /// This is because, the pager reserves only one line for showing the prompt
    /// and a newline will cause it to span multiple lines, breaking the display
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.send_message("An error occurred");
    /// ```
    pub fn send_message(&mut self, text: impl Into<String>) {
        let message = text.into();
        if message.contains('\n') {
            panic!("Prompt text cannot contain newlines");
        }
        self.message.0 = Some(wrap_str(&message, self.cols));
        self.message.1 = true;
    }

    /// Set the prompt displayed at the prompt to `t`
    ///
    /// # Panics
    /// This function panics if the given text contains newline characters.
    /// This is because, the pager reserves only one line for showing the prompt
    /// and a newline will cause it to span multiple lines, breaking the display
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let mut pager = Pager::new().unwrap();
    /// pager.set_prompt("my awesome program");
    /// ```
    pub fn set_prompt(&mut self, t: impl Into<String>) {
        let prompt = t.into();
        if prompt.contains('\n') {
            panic!("Prompt text cannot contain newlines");
        }
        self.prompt = wrap_str(&prompt, self.cols);
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
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "tokio_lib", feature = "async_std_lib")))
    )]
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
    pub fn push_str(&mut self, string: impl Into<String>) {
        let text = string.into();

        // if the text we have saved currently ends with a newline,
        // we want the formatted_text vector to append the line instead of
        // trying to add it to the last item
        let newline = self.lines.ends_with('\n');

        // find the number of trailing whitespace characters currently on self.lines
        let ending_whitespace = self
            .lines
            .chars()
            .rev()
            .take_while(|c| c.is_whitespace() && *c != '\n')
            .collect::<String>();

        // push the text to lines
        self.lines.push_str(&text);

        // And get how many lines of text will be shown (not how many rows, how many wrapped
        // lines), and get its string length
        let len_line_number = self.lines.lines().count().to_string().len();

        // if we want a newline, just format the new text and append it.
        // if we don't, format the text with the last line currently formatted
        // since it will be appended to that
        //
        // also get the line number to start at when formatting
        let (to_format, to_skip) = if newline {
            (text, self.lines.lines().count())
        } else {
            // add the trailing whitespace in here, since it isn't preserved when wrapping the
            // lines, and thus won't appear on the last element in self.formatted_lines
            let to_fmt = format!(
                "{}{}{}",
                self.formatted_lines.pop().unwrap_or_default(),
                ending_whitespace,
                text
            );

            (to_fmt, self.lines.lines().count().saturating_sub(1))
        };

        // format the lines we want to format
        let mut to_append = to_format
            .lines()
            .enumerate()
            .flat_map(|(idx, line)| {
                self.formatted_line(
                    line,
                    matches!(
                        self.line_numbers,
                        LineNumbers::AlwaysOn | LineNumbers::Enabled
                    ),
                    len_line_number,
                    idx + to_skip,
                )
            })
            .collect::<Vec<String>>();

        // append the new vector to the formatted lines
        self.formatted_lines.append(&mut to_append);
    }

    /// Hints the running pager that no more data is coming
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

    pub(crate) fn formatted_line(
        &self,
        line: &str,
        line_numbers: bool,
        len_line_number: usize,
        idx: usize,
    ) -> Vec<String> {
        if line_numbers {
            #[cfg_attr(not(feature = "search"), allow(unused_mut))]
            wrap_str(line, self.cols.saturating_sub(len_line_number + 3))
                .into_iter()
                .map(|mut row| {
                    #[cfg(feature = "search")]
                    if let Some(st) = self.search_term.as_ref() {
                        // highlight the lines with matching search terms
                        row = search::highlight_line_matches(&row, st);
                    }

                    if cfg!(not(test)) {
                        format!(
                            " {bold}{number: >len$}.{reset} {row}",
                            bold = crossterm::style::Attribute::Bold,
                            number = idx + 1,
                            len = len_line_number,
                            reset = crossterm::style::Attribute::Reset,
                            row = row
                        )
                    } else {
                        format!(
                            " {number: >len$}. {row}",
                            number = idx + 1,
                            len = len_line_number,
                            row = row
                        )
                    }
                })
                .collect::<Vec<String>>()
        } else {
            #[cfg(feature = "search")]
            if let Some(st) = self.search_term.as_ref() {
                return wrap_str(&search::highlight_line_matches(line, st), self.cols);
            }

            wrap_str(line, self.cols)
        }
    }

    pub(crate) fn format_lines(&mut self) {
        let line_count = self.lines.lines().count();

        let len_line_number = line_count.to_string().len();

        self.formatted_lines = self
            .lines
            .lines()
            .enumerate()
            .flat_map(|(idx, line)| {
                self.formatted_line(
                    line,
                    matches!(
                        self.line_numbers,
                        LineNumbers::AlwaysOn | LineNumbers::Enabled
                    ),
                    len_line_number,
                    idx,
                )
            })
            .collect::<Vec<String>>();

        if self.message.0.is_some() {
            rewrap(&mut self.message.0.as_mut().unwrap(), self.cols);
        }
        rewrap(&mut self.prompt, self.cols);

        #[cfg(feature = "search")]
        search::set_match_indices(self);
    }

    /// Returns all the text within the bounds, after flattening
    pub(crate) fn get_flattened_lines_with_bounds(&self, start: usize, end: usize) -> &[String] {
        if start >= self.num_lines() || start > end {
            &[]
        } else if end >= self.num_lines() {
            &self.formatted_lines[start..]
        } else {
            &self.formatted_lines[start..end]
        }
    }

    /// Returns the number of lines the [`Pager`] currently holds
    pub(crate) fn num_lines(&self) -> usize {
        self.formatted_lines.len()
    }

    /// Set custom input handler function
    ///
    /// See example in [`InputHandler`](input::InputClassifier) on using this
    /// function
    pub fn set_input_handler(&mut self, handler: Box<dyn input::InputClassifier + Send + Sync>) {
        self.input_classifier = handler;
    }

    // Runs the exit callbacks
    pub(crate) fn exit(&mut self) {
        for func in &mut self.exit_callbacks {
            func();
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

impl fmt::Write for Pager {
    fn write_str(&mut self, string: &str) -> fmt::Result {
        self.push_str(string);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
