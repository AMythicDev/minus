use std::collections::VecDeque;

pub mod commands;
pub mod ev_handler;
#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
pub mod init;
pub mod utils;
pub static RUNMODE: parking_lot::Mutex<RunMode> = parking_lot::const_mutex(RunMode::Uninitialized);

use commands::Command;

pub struct CommandQueue(VecDeque<Command>);

impl CommandQueue {
    pub fn new() -> Self {
        Self(VecDeque::with_capacity(10))
    }
    pub fn new_zero() -> Self {
        Self(VecDeque::with_capacity(0))
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn push_back_unchecked(&mut self, value: Command) {
        self.0.push_back(value);
    }
    pub fn push_back(&mut self, value: Command) {
        if RUNMODE.lock().is_uninitialized() {
            panic!();
        }
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
