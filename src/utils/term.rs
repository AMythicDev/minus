// Terminal functions
// Contains functions for dealing with setup, cleanup

use crate::error::{CleanupError, SetupError};
use crossterm::{cursor, event, execute, terminal};
use std::io;

// This function should be kept close to `cleanup` to help ensure both are
// doing the opposite of the other.
//
// Setup the terminal and get the necessary informations.
//
// `dynamic` tells whether `minus` will do a dynamic paging or static paging.
// When `dynamic` is set to true, `minus` wll exit with an error if the stdout is nt
// a TTY.
//
// ## Errors
//
// Setting up the terminal can fail, see [`SetupError`](SetupError).
pub(crate) fn setup(
    stdout: &io::Stdout,
    dynamic: bool,
    setup_screen: bool,
) -> std::result::Result<(), SetupError> {
    let mut out = stdout.lock();

    if setup_screen {
        // Check if the standard output is a TTY and not a file or something else but only in dynamic mode
        if dynamic {
            use crossterm::tty::IsTty;

            if out.is_tty() {
                Ok(())
            } else {
                Err(SetupError::InvalidTerminal)
            }?;
        }

        execute!(out, terminal::EnterAlternateScreen)
            .map_err(|e| SetupError::AlternateScreen(e.into()))?;
        terminal::enable_raw_mode().map_err(|e| SetupError::RawMode(e.into()))?;
        execute!(out, cursor::Hide).map_err(|e| SetupError::HideCursor(e.into()))?;
    }
    Ok(())
}

// Will try to clean up the terminal and set it back to its original state,
// before the pager was setup and called.
//
// Use this function if you encounter problems with your application not
// correctly setting back the terminal on errors.
//
// ## Errors
//
// Cleaning up the terminal can fail, see [`CleanupError`](CleanupError).
pub(crate) fn cleanup(
    mut out: impl io::Write,
    es: &crate::ExitStrategy,
    cleanup_screen: bool,
) -> std::result::Result<(), CleanupError> {
    if cleanup_screen {
        // Reverse order of setup.
        execute!(out, event::DisableMouseCapture)
            .map_err(|e| CleanupError::DisableMouseCapture(e.into()))?;
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
