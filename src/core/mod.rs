use std::collections::VecDeque;

pub mod commands;
pub mod ev_handler;
#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
pub mod init;
pub mod utils;
pub static RUNMODE: parking_lot::Mutex<RunMode> = parking_lot::const_mutex(RunMode::Uninitialized);

use commands::Command;

/// A [VecDeque] to hold [Command]s to be executed after the current command has been executed
///
/// Many [Command]s in minus require additional commands to be executed once the current command's
/// main objective has ben completed. For example the [SetLineNumbers](Command::SetLineNumbers)
/// requires the text data to be reformatted and repainted on the screen. Hence it can push that
/// command to this to be executed once it itself has completed executing.
///
/// This also takes into account [RUNMODE] before inserting data. The means that it will ensure that
/// [RUNMODE] is not uninitialized before pushing any data into the queue. Hence it is best used
/// case is while declaring handlers for [Command::UserInput].
///
/// This is a FIFO type hence the command that enters first gets executed first.
pub struct CommandQueue(VecDeque<Command>);

impl CommandQueue {
    /// Create a new CommandQueue with default size of 10.
    pub fn new() -> Self {
        Self(VecDeque::with_capacity(10))
    }
    /// Create a new CommandQueue with zero memory allocation.
    ///
    /// This is useful when we have to pass this type to [handle_event](ev_handler::handle_event)
    /// but it is sure that this won't be used.
    pub fn new_zero() -> Self {
        Self(VecDeque::with_capacity(0))
    }
    /// Returns true if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Store `value` only if [RUNMODE] is not unintialized.
    ///
    /// # Panics
    /// This function will panic if it is called in an environment where [RUNMODE] is
    /// uninitialized.
    pub fn push_back(&mut self, value: Command) {
        assert!(!RUNMODE.lock().is_uninitialized(), "CommandQueue::push_back() caled when  RUNMODE is not set. This is most likely a bug. Please report the issue on minus's issue tracker on Github.");
        self.0.push_back(value);
    }
    /// Store `value` without checking [RUNMODE].
    ///
    /// This is only meant to be used as an optimization over [push_back](CommandQueue::push_back)
    /// when it is absolutely sure that [RUNMODE] isn't uninitialized. Hence calling this in an
    /// enviroment where [RUNMODE] is uninitialized can lead to unexpect slowdowns.
    pub fn push_back_unchecked(&mut self, value: Command) {
        self.0.push_back(value);
    }
    pub fn pop_front(&mut self) -> Option<Command> {
        self.0.pop_front()
    }
}

/// Define the modes in which minus can run
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum RunMode {
    #[cfg(feature = "static_output")]
    Static,
    #[cfg(feature = "dynamic_output")]
    Dynamic,
    Uninitialized,
}

impl RunMode {
    /// Returns true if minus hasn't started
    ///
    /// # Example
    /// ```
    /// use minus::RunMode;
    ///
    /// let runmode = RunMode::Uninitialized;
    /// assert_eq!(runmode.is_uninitialized(), true);
    /// ```
    #[must_use]
    pub fn is_uninitialized(self) -> bool {
        self == Self::Uninitialized
    }
}
