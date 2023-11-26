pub mod commands;
pub mod ev_handler;
#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
pub mod init;
pub mod utils;
pub static RUNMODE: parking_lot::Mutex<RunMode> = parking_lot::const_mutex(RunMode::Uninitialized);

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
