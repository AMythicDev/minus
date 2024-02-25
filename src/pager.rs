//! Proivdes the [Pager] type

use crate::{error::MinusError, input, minus_core::commands::Command, ExitStrategy, LineNumbers};
use crossbeam_channel::{Receiver, Sender};
use std::fmt;

#[cfg(feature = "search")]
use crate::search::SearchOpts;

/// A communication bridge between the main application and the pager.
///
/// The [Pager] type which is a bridge between your application and running
/// the running pager. Its the single most important type with which you will be interacting the
/// most while working with minus. It allows you to send data, configure UI settings and also
/// configure the key/mouse bindings.
///
/// You can
/// - send data and
/// - set configuration options
///
/// before or while the pager is running.
///
/// [Pager] also implements the [std::fmt::Write] trait which means you can directly call [write!] and
/// [writeln!] macros on it. For example, you can easily do this
///
/// ```
/// use minus::Pager;
/// use std::fmt::Write;
///
/// const WHO: &str = "World";
/// let mut pager = Pager::new();
///
/// // This appends `Hello World` to the end of minus's buffer
/// writeln!(pager, "Hello {WHO}").unwrap();
/// // which is also equivalent to writing this
/// pager.push_str(format!("Hello {WHO}\n")).unwrap();
#[derive(Clone)]
pub struct Pager {
    pub(crate) tx: Sender<Command>,
    pub(crate) rx: Receiver<Command>,
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
        Ok(self.tx.send(Command::SetData(s.into()))?)
    }

    /// Appends text to the pager output.
    ///
    /// You can also use [`write!`]/[`writeln!`] macros to append data to the pager.
    /// The implementation basically calls this function internally. One difference
    /// between using the macros and this function is that this does not require `Pager`
    /// to be declared mutable while in order to use the macros, you need to declare
    /// the `Pager` as mutable.
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
        Ok(self.tx.send(Command::AppendData(s.into()))?)
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
    /// pager.set_line_numbers(LineNumbers::Enabled).expect("Failed to communicate with the pager");
    /// ```
    pub fn set_line_numbers(&self, l: LineNumbers) -> Result<(), MinusError> {
        Ok(self.tx.send(Command::SetLineNumbers(l))?)
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
        let text: String = text.into();
        assert!(!text.contains('\n'), "Prompt cannot contain newlines");
        Ok(self.tx.send(Command::SetPrompt(text))?)
    }

    /// Send a message to be displayed the prompt area
    ///
    /// The text message is temporary and will get cleared whenever the use
    /// rdoes a action on the terminal like pressing a key or scrolling using the mouse.
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
        let text: String = text.into();
        assert!(!text.contains('\n'), "Message cannot contain newlines");
        Ok(self.tx.send(Command::SendMessage(text))?)
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
    /// pager.set_exit_strategy(ExitStrategy::ProcessQuit).expect("Failed to communicate with the pager");
    /// ```
    pub fn set_exit_strategy(&self, es: ExitStrategy) -> Result<(), MinusError> {
        Ok(self.tx.send(Command::SetExitStrategy(es))?)
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
    /// pager.set_run_no_overflow(true).expect("Failed to communicate with the pager");
    /// ```
    #[cfg(feature = "static_output")]
    #[cfg_attr(docsrs, doc(cfg(feature = "static_output")))]
    pub fn set_run_no_overflow(&self, val: bool) -> Result<(), MinusError> {
        Ok(self.tx.send(Command::SetRunNoOverflow(val))?)
    }

    /// Whether to allow scrolling horizontally
    ///
    /// Setting this to `true` implicitly disables line wrapping
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new();
    /// pager.horizontal_scroll(true).expect("Failed to communicate with the pager");
    /// ```
    pub fn horizontal_scroll(&self, value: bool) -> Result<(), MinusError> {
        Ok(self.tx.send(Command::LineWrapping(!value))?)
    }

    /// Set a custom input classifer type.
    ///
    /// An input classifier type is a type that implements the [InputClassifier]
    /// trait. It only has one required function, [InputClassifier::classify_input]
    /// which matches user input events and maps them to a [InputEvent]s.
    /// When the pager encounters a user input, it calls the input classifier with
    /// the event and [PagerState] as parameters.
    ///
    /// Previously, whenever any application wanted to change the default key/mouse bindings
    /// they neededd to create a new type, implement the [InputClassifier] type by copying and
    /// pasting the default minus's implementation of it available in the [DefaultInputClassifier]
    /// and change the parts they wanted to change. This is not only unergonomic but also
    /// extreemely prone to bugs. Hence a newer and much simpler method was developed.
    /// This method is still allowed to avoid breaking backwards compatiblity but will be dropped
    /// in the next major release.
    ///
    /// With the newer method, minus already provides a type called [HashedEventRegister]
    /// which implementing the [InputClassifier] and is based on a
    /// [HashMap] storing all the key/mouse bindings and its associated callback function.
    /// This allows easy addition/updation/deletion of the default bindings with simple functions
    /// like [HashedEventRegister::add_key_events] and [HashedEventRegister::add_mouse_events]
    ///
    /// See the [input] module for information about implementing it.
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the receiver
    ///
    /// [HashedEventRegister::add_key_events]: input::HashedEventRegister::add_key_events
    /// [HashedEventRegister::add_mouse_events]: input::HashedEventRegister::add_mouse_events
    /// [HashMap]: std::collections::HashMap
    /// [PagerState]: crate::state::PagerState
    /// [InputEvent]: input::InputEvent
    /// [InputClassifier]: input::InputClassifier
    /// [InputClassifier::classify_input]: input::InputClassifier
    /// [HashedEventRegister]: input::HashedEventRegister
    /// [DefaultInputClassifier]: input::DefaultInputClassifier
    pub fn set_input_classifier(
        &self,
        handler: Box<dyn input::InputClassifier + Send + Sync>,
    ) -> Result<(), MinusError> {
        Ok(self.tx.send(Command::SetInputClassifier(handler))?)
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
    /// pager.add_exit_callback(Box::new(hello)).expect("Failed to communicate with the pager");
    /// ```
    pub fn add_exit_callback(
        &self,
        cb: Box<dyn FnMut() + Send + Sync + 'static>,
    ) -> Result<(), MinusError> {
        Ok(self.tx.send(Command::AddExitCallback(cb))?)
    }

    /// Override the condition for running incremental search
    ///
    /// See [Incremental Search](../search/index.html#incremental-search) to know more on how this
    /// works
    ///
    /// # Errors
    /// This function will returns a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be send to the receiver end.
    #[cfg(feature = "search")]
    #[cfg_attr(docsrs, doc(cfg(feature = "search")))]
    pub fn set_incremental_search_condition(
        &self,
        cb: Box<dyn Fn(&SearchOpts) -> bool + Send + Sync + 'static>,
    ) -> crate::Result {
        self.tx.send(Command::IncrementalSearchCondition(cb))?;
        Ok(())
    }

    /// Control whether to show the prompt
    ///
    /// Many applications don't want the prompt to be displayed at all. This function can be used to completely turn
    /// off the prompt. Passing `false` to this will stops the prompt from displaying and instead a blank line will
    /// be displayed.
    ///
    /// Note that This merely stop the prompt from being shown. Your application can still update the
    /// prompt and send messages to the user but it won't be shown until the prompt isn't re-enabled.
    /// The prompt section will also be used when user opens the search prompt to type a search query.
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the mus's receiving end
    ///
    /// # Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new();
    /// pager.show_prompt(false).unwrap();
    /// ```
    pub fn show_prompt(&self, show: bool) -> crate::Result {
        self.tx.send(Command::ShowPrompt(show))?;
        Ok(())
    }

    /// Configures follow output
    ///
    /// When set to true, minus ensures that the user's screen always follows the end part of the
    /// output. By default it is turned off.
    ///
    /// This is similar to [InputEvent::FollowOutput](crate::input::InputEvent::FollowOutput) except that
    /// this is used to control it from the application's side.
    ///
    /// # Errors
    /// This function will return a [`Err(MinusError::Communication)`](MinusError::Communication) if the data
    /// could not be sent to the mus's receiving end
    ///
    /// # Example
    /// ```
    /// use minus::Pager;
    ///
    /// let pager = Pager::new();
    /// pager.follow_output(true).unwrap();
    /// ```
    pub fn follow_output(&self, follow_output: bool) -> crate::Result {
        self.tx.send(Command::FollowOutput(follow_output))?;
        Ok(())
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
