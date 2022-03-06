//! Contains functions for dealing with setup, cleanup

use crate::error::{CleanupError, SetupError};
use crossterm::{cursor, execute, terminal, tty::IsTty};
use std::io;

/// Setup the terminal
///
/// It will
/// - Switch the terminal's view to the [alternate screen]
/// - Then enable [raw mode]
/// - Clear the entire screen and hide the cursor.
///
/// # Errors
/// The function will return with an error if `stdout` is not a terminal. It will qlso fail
/// if it cannot executo commands on the terminal See [`SetupError`].
///
/// [alternate screen]: ../../../crossterm/terminal/index.html#alternate-screen
/// [raw mode]: ../../../crossterm/terminal/index.html#raw-mode
// This function should be kept close to `cleanup` to help ensure both are
// doing the opposite of the other.
pub(crate) fn setup(stdout: &io::Stdout) -> std::result::Result<(), SetupError> {
    let mut out = stdout.lock();

    if out.is_tty() {
        Ok(())
    } else {
        Err(SetupError::InvalidTerminal)
    }?;

    execute!(out, terminal::EnterAlternateScreen)
        .map_err(|e| SetupError::AlternateScreen(e.into()))?;
    terminal::enable_raw_mode().map_err(|e| SetupError::RawMode(e.into()))?;
    execute!(out, cursor::Hide).map_err(|e| SetupError::HideCursor(e.into()))?;
    Ok(())
}

/// Cleans up the terminal
///
/// The function will clean up the terminal and set it back to its original state,
/// before the pager was setup and called.
/// - First the cursor is displayed
/// - [Raw mode] is disabled
/// - Switch the terminal's view to the main screen
///
/// ## Errors
/// The function will return with an error if it fails to do execute commands on the
/// terminal. See [`CleanupError`]
///
/// [raw mode]: ../../../crossterm/terminal/index.html#raw-mode
pub(crate) fn cleanup(
    mut out: impl io::Write,
    es: &crate::ExitStrategy,
    cleanup_screen: bool,
) -> std::result::Result<(), CleanupError> {
    if cleanup_screen {
        // Reverse order of setup.
        execute!(out, cursor::Show).map_err(|e| CleanupError::ShowCursor(e.into()))?;
        terminal::disable_raw_mode().map_err(|e| CleanupError::DisableRawMode(e.into()))?;
        execute!(out, terminal::LeaveAlternateScreen)
            .map_err(|e| CleanupError::LeaveAlternateScreen(e.into()))?;
    }

    if *es == crate::ExitStrategy::ProcessQuit {
        std::process::exit(0);
    } else {
        Ok(())
    }
}
