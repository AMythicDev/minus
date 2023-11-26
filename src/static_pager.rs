//! Contains function for displaying static data
//!
//! This module provides provides the [`page_all`] function to display static output via minus
use crate::minus_core::init;
use crate::{error::MinusError, Pager};

/// Display static information to the screen
///
/// Since it is sure that fed data will never change, minus can do some checks like:-
/// * If stdout is not a tty, minus not start a pager. It will simply print all the data and quit
/// * If there are more rows in the terminal than the number of lines of data to display
/// minus will not start a pager and simply display all data on the main stdout screen.
/// This behaviour can be turned off if
/// [`Pager::set_run_no_overflow(true)`](Pager::set_run_no_overflow) has been
/// called before starting
/// * Since any other event except user inputs will not occur, we can do some optimizations on
/// matching events.
///
/// See [example](../index.html#static-output) on how to use this function.
///
/// # Panics
/// This function will panic if another instance of minus is already running.
///
/// # Errors
/// The function will return with an error if it encounters a error during paging.
#[cfg_attr(docsrs, doc(cfg(feature = "static_output")))]
#[allow(clippy::needless_pass_by_value)]
pub fn page_all(pager: Pager) -> Result<(), MinusError> {
    init::init_core(&pager, crate::RunMode::Static)
}
