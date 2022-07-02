use crate::{error::MinusError, input, minus_core::events::Event, ExitStrategy, LineNumbers};
use crossbeam_channel::{Receiver, Sender};
use std::fmt;

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
/// the [PagerState](crate::state::PagerState)
#[derive(Clone)]
pub struct Pager {
    pub(crate) tx: Sender<Event>,
    pub(crate) rx: Receiver<Event>,
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
    #[cfg_attr(docsrs, doc(cfg(feature = "static_output")))]
    pub fn set_run_no_overflow(&self, val: bool) -> Result<(), MinusError> {
        Ok(self.tx.send(Event::SetRunNoOverflow(val))?)
    }

    /// Set a custom input classifer function.
    ///
    /// When the pager encounters a user input, it calls the input classifer with
    /// the event and [PagerState](crate::state::PagerState) as parameters.
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

impl fmt::Write for Pager {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).map_err(|_| fmt::Error)
    }
}
