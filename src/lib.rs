#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    not(any(feature = "async_output", feature = "static_output")),
    allow(unused_imports),
    allow(dead_code)
)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::doc_markdown)]

//! minus is an asynchronous terminal paging library written in Rust.
//!
//! # What is a Pager?
//! A pager is a program that lets you view and scroll through large amounts of text using a keyboard in a TTY where no mouse support is available.
//!
//! Nowadays most people use a graphical terminals where mouse support is present but they aren't as reliable as a pager. For example they may not support proper text searching or line numbering, plus quick navigation using keyboard is pretty much non-existent.
//!
//! Examples of some popular pager include `more` and its successor `less`.
//!
//! # The problem with traditional pagers
//! First, traditional pagers like `more` or `less` weren't made for integrating into other applications. They were meant to be standalone binaries that are executed directly by the users.
//!
//! Applications leveraged these pagers by calling them as external programs and passing the data
//! through the standard input. This method worked for Unix and other Unix-like OSs like Linux and
//! MacOS because they already came with any of these pagers installed  But it wasn't this eas
//! y on Windows, it required shipping the pager binary along with the applications. Since these p
//! rograms were originally designed for Unix and Unix-like OSs, distributing these binaries meant
//! shipping an entire environment like MinGW or Cygwin so that these can run properly on Windows.
//!
//!  Recently, some libraries have emerged to solve this issue. They are compiled along with your application and give you a single to distribute. The problem with this is most of them require you to feed the entire data to the pager before the pager can run, this meant that there will be no output on the terminal until the entire data isn't loaded by the application and passed on to the pager.
//!
//! These could cause long delays before output if the data comes from a very large file or is being downloaded from the internet.
//!
//! # Enter minus
//! As above described, minus is an asynchronous terminal paging library for Rust. It allows not just data but also configuration to be fed into itself while it is running.
//!
//! minus achieves this by using Rust's amazing concurrency support and no data race conditions which are guaranteed by Rust
//!
//! With minus, you can initialize a pager with any async runtime of your choice or even no runtime, if you want just native threads.
//!
//! # Usage
//! Add minus as a dependency in your `Cargo.toml` file and enable features as you like.
//! * If you only want a pager to display static data, enable the `static_output` feature
//! * If you want a pager to display dynamic data and be configurable at runtime, enable the `async_output` feature
//! * If you want search support inside the pager, you need to enable the `search` feature
//! ```toml
//! [dependencies.minus]
//! version = "^5.0"
//! features = [
//!    # Enable features you want. For example
//!    "async_output",
//!    "search"
//! ]
//! ```

//! # Examples

//! All example are available in the `examples` directory and you can run them using `cargo`.
//!
//! ## [`tokio`]
//!
//! ```rust,no_run
//! use minus::{async_paging, MinusError, Pager};
//! use std::time::Duration;
//! use std::fmt::Write;
//! use tokio::{join, spawn, time::sleep};
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
//!     // spawn(async_paging(...)) creates a tokio task and runs the async_paging
//!     // inside it
//!     let (res1, res2) = join!(spawn(async_paging(pager.clone())), increment);
//!     // .unwrap() unwraps any error while creating the tokio task
//!     //  The ? mark unpacks any error that might have occured while the
//!     // pager is running
//!     res1.unwrap()?;
//!     res2?;
//!     Ok(())
//! }
//! ```
//!
//! ## [`async-std`]:
//!
//! ```rust,no_run
//! use async_std::task::{sleep, spawn};
//! use futures_lite::future;
//! use minus::{async_paging, MinusError, Pager};
//! use std::time::Duration;
//!
//! #[async_std::main]
//! async fn main() -> Result<(), MinusError> {
//!     let output = Pager::new();
//!
//!     let increment = async {
//!         for i in 0..=100_u32 {
//!             output.push_str(&format!("{}\n", i))?;
//!             sleep(Duration::from_millis(100)).await;
//!         }
//!         Result::<_, MinusError>::Ok(())
//!     };
//!
//!     let (res1, res2) = future::zip(spawn(async_paging(output.clone())), increment).await;
//!     res1?;
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
//! | Action            | Description                                        |
//! | ----------------- | -------------------------------------------------- |
//! | Ctrl+C/q          | Quit the pager                                     |
//! | Arrow Up/k        | Scroll up by one line                              |
//! | Arrow Down/j      | Scroll down by one line                            |
//! | Page Up           | Scroll up by entire page                           |
//! | Page Down         | Scroll down by entire page                         |
//! | Enter             | Scroll down by one line or clear prompt messages   |
//! | Space             | Scroll down by one page                            |
//! | Ctrl+U/u          | Scroll up by half a screen                         |
//! | Ctrl+D/d          | Scroll down by half a screen                       |
//! | g                 | Go to the very top of the output                   |
//! | G                 | Go to the very bottom of the output                |
//! | Mouse scroll Up   | Scroll up by 5 lines                               |
//! | Mouse scroll Down | Scroll down by 5 lines                             |
//! | Ctrl+L            | Toggle line numbers if not forced enabled/disabled |
//! | /                 | Start forward search                               |
//! | ?                 | Start backward search                              |
//! | Esc               | Cancel search input                                |
//! | n                 | Go to the next search match                        |
//! | p                 | Go to the next previous match                      |
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

// When no feature is active this crate is unusable but contains lots of
// unused imports and dead code. To avoid useless warnings about this they
// are allowed when no feature is active.

#[cfg(feature = "dynamic_output")]
mod dynamic_pager;
pub mod error;
pub mod input;
#[path = "core/mod.rs"]
mod minus_core;
#[cfg(feature = "static_output")]
mod static_pager;

#[cfg(feature = "static_output")]
pub use static_pager::page_all;
#[cfg(feature = "dynamic_output")]
pub use dynamic_pager::dynamic_paging;

use crossbeam_channel::{Receiver, Sender};
use crossterm::{terminal, tty::IsTty};
pub use error::MinusError;
use error::TermError;
use minus_core::events::Event;
#[cfg(feature = "search")]
use minus_core::search;
#[cfg(feature = "search")]
pub use minus_core::search::SearchMode;
use std::string::ToString;
use std::{fmt, io::stdout};

/// A convenient type for `Vec<Box<dyn FnMut() + Send + Sync + 'static>>`
pub type ExitCallbacks = Vec<Box<dyn FnMut() + Send + Sync + 'static>>;

/// A pager acts as a middleman for communication between the main application
/// and the user with the core functions of minus
///
/// It consists of a [`crossbeam_channel::Sender`] and  [`crossbeam_channel::Receiver`]
/// pair. When a method like [`set_text`](Pager::set_text) or [`push_str`](Pager::push_str)
/// is called, the function takes the input. wraps it in the appropriate event
/// type and transmits it through the sender held inside the this.
///
/// The receiver part of the channel is continously polled by the pager for events. Depending
/// on the type of event that occurs, the pager will either redraw the screen or update
/// the [`PagerState`]
#[derive(Clone)]
pub struct Pager {
    tx: Sender<Event>,
    rx: Receiver<Event>,
}

impl Pager {
    /// Initialize a new pager
    ///
    /// # Example
    /// ```
    /// let pager = minus::Pager::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        Self { tx, rx }
    }

    /// Set the output text to this `t`
    ///
    /// Note that unlike [`Pager::push_str`], this replaces the original text.
    /// If you want to append text, use the [`Pager::push_str`] function or the
    /// [`write!`]/[`writeln!`] macros
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// # Example
    /// ```
    /// let pager = minus::Pager::new();
    /// pager.set_text("This is a line").expect("Failed to send data to the pager");
    /// ```
    pub fn set_text(&self, s: impl Into<String>) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::SetData(s.into()))?)
    }

    /// Appends text to the pager output.
    ///
    /// You can also use [`write!`]/[`writeln!`] macros to append data to the pager.
    /// The implementation basically calls this function internally.
    ///
    /// One difference between using the macros and this function is that this does
    /// not require `Pager` to be declared mutable while in order to use the macros,
    /// you need to declare the `Pager` as mutable.
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// # Example
    /// ```
    /// use std::fmt::Write;
    ///
    /// let mut pager = minus::Pager::new();
    /// pager.push_str("This is some text").expect("Failed to send data to the pager");
    /// // This is same as above
    /// write!(pager, "This is some text").expect("Failed to send data to the pager");
    /// ```
    pub fn push_str(&self, s: impl Into<String>) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::AppendData(s.into()))?)
    }

    /// Set line number configuration for the pager
    ///
    /// See [`LineNumbers`] for available options
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// # Example
    /// ```
    /// use minus::{Pager, LineNumbers};
    ///
    /// let pager = Pager::new();
    /// pager.set_line_numbers(LineNumbers::Enabled).expect("Failed to send data to the pager");
    /// ```
    pub fn set_line_numbers(&self, l: LineNumbers) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::SetLineNumbers(l))?)
    }

    /// Set the text displayed at the bottom prompt
    ///
    /// # Panics
    /// This function panics if the given text contains newline characters.
    /// This is because, the pager reserves only one line for showing the prompt
    /// and a newline will cause it to span multiple lines, breaking the display
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new();
    /// pager.set_prompt("my prompt").expect("Failed to send data to the pager");
    /// ```
    pub fn set_prompt(&self, text: impl Into<String>) -> Result<(), MinusError> {
        let text = text.into();
        assert!(!text.contains('\n'), "Prompt cannot contain newlines");
        Ok(self.tx.send(Event::SetPrompt(text))?)
    }

    /// Display a temporary message at the prompt area
    ///
    /// # Panics
    /// This function panics if the given text contains newline characters.
    /// This is because, the pager reserves only one line for showing the prompt
    /// and a newline will cause it to span multiple lines, breaking the display
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// # Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new();
    /// pager.send_message("An error occurred").expect("Failed to send data to the pager");
    /// ```
    pub fn send_message(&self, text: impl Into<String>) -> Result<(), MinusError> {
        let text = text.into();
        assert!(!text.contains('\n'), "Message cannot contain newlines");
        Ok(self.tx.send(Event::SendMessage(text))?)
    }

    /// Set the default exit strategy.
    ///
    /// This controls how the pager will behave when the user presses `q` or `Ctrl+C`.
    /// See [`ExitStrategy`] for available options
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// ```
    /// use minus::{Pager, ExitStrategy};
    ///
    /// let pager = Pager::new();
    /// pager.set_exit_strategy(ExitStrategy::ProcessQuit).expect("Failed to send data to the pager");
    /// ```
    pub fn set_exit_strategy(&self, es: ExitStrategy) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::SetExitStrategy(es))?)
    }

    /// Set whether to display pager if there's less data than
    /// available screen height
    ///
    /// When this is set to false, the pager will simply print all the lines
    /// to the main screen and immediately quit if the number of lines to
    /// display is less than the available columns in the terminal.
    /// Setting this to true will cause a full pager to start and display the data
    /// even if there is less number of lines to display than available rows.
    ///
    /// This is only available in static output mode as the size of the data is
    /// known beforehand.
    /// In async output the pager can receive more data anytime
    ///
    /// By default this is set to false
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new();
    /// pager.set_run_no_overflow(true).expect("Failed to send data to the pager");
    /// ```
    #[cfg(feature = "static_output")]
    #[cfg_attr(docsrs, feature = "static_output")]
    pub fn set_run_no_overflow(&self, val: bool) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::SetRunNoOverflow(val))?)
    }

    /// Set a custom input classifer function.
    ///
    /// When the pager encounters a user input, it calls the input classifer with
    /// the event and [`PagerState`] as parameters.
    ///
    /// A input classifier is a type implementing the [`InputClassifier`](input::InputClassifier)
    /// trait. It only has one required function, [`InputClassifier::classify_input`](input::InputClassifier::classify_input)
    /// which matches user input events and maps them to a [`InputEvent`](input::InputEvent)s.
    ///
    /// See the [`InputHandler`](input::InputClassifier) trait for information about implementing
    /// it.
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    pub fn set_input_classifier(
        &self,
        handler: Box<dyn input::InputClassifier + Send + Sync>,
    ) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::SetInputClassifier(handler))?)
    }

    /// Adds a function that will be called when the user quits the pager
    ///
    /// Multiple functions can be stored for calling when the user quits. These functions
    /// run sequentially in the order they were added
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// # Example
    /// ```
    /// use minus::Pager;
    ///
    /// fn hello() {
    ///     println!("Hello");
    /// }
    ///
    /// let pager = Pager::new();
    /// pager.add_exit_callback(Box::new(hello)).expect("Failed to send data to the pager");
    /// ```
    pub fn add_exit_callback(
        &self,
        cb: Box<dyn FnMut() + Send + Sync + 'static>,
    ) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::AddExitCallback(cb))?)
    }
}

impl Default for Pager {
    fn default() -> Self {
        Self::new()
    }
}

/// Holds all information and configuration about the pager during
/// its un time.
///
/// This type is exposed so that end-applications can implement the
/// [`InputClassifier`](input::InputClassifier) trait which requires the `PagerState` to be passed
/// as a parameter
///
/// Various fields are made public so that their values can be accessed while implementing the
/// trait.
pub struct PagerState {
    /// The text the pager has been told to be displayed
    lines: String,
    /// The output, flattened and formatted into the lines that should be displayed
    formatted_lines: Vec<String>,
    /// Configuration for line numbers. See [`LineNumbers`]
    pub line_numbers: LineNumbers,
    /// The prompt displayed at the bottom wrapped to available terminal width
    prompt: Vec<String>,
    /// The input classifier to be called when a input is detected
    input_classifier: Box<dyn input::InputClassifier + Sync + Send>,
    /// Functions to run when the pager quits
    exit_callbacks: Vec<Box<dyn FnMut() + Send + Sync + 'static>>,
    /// The behaviour to do when user quits the program using `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    exit_strategy: ExitStrategy,
    /// Any message to display to the user at the prompt
    /// The first element contains the actual message, while the second element tells
    /// whether the message has changed since the last display.
    message: Option<Vec<String>>,
    /// The upper bound of scrolling.
    ///
    /// This is useful for keeping track of the range of lines which are currently being displayed on
    /// the terminal.
    /// When `rows - 1` is added to the `upper_mark`, it gives the lower bound of scroll.
    ///
    /// For example if there are 10 rows is a terminal and the data to display has 50 lines in it/
    /// If the `upper_mark` is 15, then the first row of the terminal is the 16th line of the data
    /// and last row is the 24th line of the data.
    pub upper_mark: usize,
    /// Do we want to page if there is no overflow
    #[cfg(feature = "static_output")]
    run_no_overflow: bool,
    /// Stores the most recent search term
    #[cfg(feature = "search")]
    search_term: Option<regex::Regex>,
    /// Direction of search
    ///
    /// See [`SearchMode`] for available options
    #[cfg(feature = "search")]
    #[cfg_attr(docsrs, feature = "search")]
    pub search_mode: SearchMode,
    /// Lines where searches have a match
    #[cfg(feature = "search")]
    search_idx: Vec<usize>,
    /// Index of search item currently in focus
    /// It should be 0 even when no search is in action
    #[cfg(feature = "search")]
    search_mark: usize,
    /// Available rows in the terminal
    pub rows: usize,
    /// Available columns in the terminal
    pub cols: usize,
    /// This variable helps in scrolling more than one line at a time
    /// It keeps track of all the numbers that have been entered by the user
    /// untill any of `j`, `k`, `G`, `Up` or `Down` is pressed
    pub prefix_num: String,
}

impl PagerState {
    pub(crate) fn new() -> Result<Self, TermError> {
        let (rows, cols);

        if cfg!(test) {
            // In tests, set  number of columns to 80 and rows to 10
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

        Ok(Self {
            lines: String::new(),
            formatted_lines: Vec::new(),
            line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            prompt: wrap_str("minus", cols),
            exit_strategy: ExitStrategy::ProcessQuit,
            input_classifier: Box::new(input::DefaultInputClassifier {}),
            exit_callbacks: Vec::new(),
            message: None,
            #[cfg(feature = "static_output")]
            run_no_overflow: false,
            #[cfg(feature = "search")]
            search_term: None,
            #[cfg(feature = "search")]
            search_mode: SearchMode::Unknown,
            #[cfg(feature = "search")]
            search_idx: Vec::new(),
            #[cfg(feature = "search")]
            search_mark: 0,
            // Just to be safe in tests, keep at 1x1 size
            cols,
            rows,
            prefix_num: String::new(),
        })
    }

    pub(crate) fn num_lines(&self) -> usize {
        self.formatted_lines.len()
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

        if self.message.is_some() {
            rewrap(self.message.as_mut().unwrap(), self.cols);
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

    /// Runs the exit callbacks
    pub(crate) fn exit(&mut self) {
        for func in &mut self.exit_callbacks {
            func();
        }
    }

    pub(crate) fn append_str(&mut self, text: &str) {
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
        self.lines.push_str(text);

        // And get how many lines of text will be shown (not how many rows, how many wrapped
        // lines), and get its string length
        let len_line_number = self.lines.lines().count().to_string().len();

        // if we want a newline, just format the new text and append it.
        // if we don't, format the text with the last line currently formatted
        // since it will be appended to that
        //
        // also get the line number to start at when formatting
        let (to_format, to_skip) = if newline {
            (text.to_owned(), self.lines.lines().count())
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
                    idx + to_skip.saturating_sub(1),
                )
            })
            .collect::<Vec<String>>();

        // append the new vector to the formatted lines
        self.formatted_lines.append(&mut to_append);
    }
}

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
    fn is_invertible(self) -> bool {
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

impl fmt::Write for Pager {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).map_err(|_| fmt::Error)
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
