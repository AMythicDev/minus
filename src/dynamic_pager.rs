use crate::error::MinusError;
use crate::minus_core::{self, init};
use crate::Pager;

/// Starts a asynchronously running pager
///
/// This means that data and configuration can be fed into the pager while it is running.
///
/// See [examples](../index.html#examples) on how to use this function.
///
/// # Panics
/// This function will panic if another instance of minus is already running.
///
/// # Errors
/// The function will return with an error if it encounters a error during paging.
#[cfg_attr(docsrs, doc(cfg(feature = "dynamic_output")))]
pub fn dynamic_paging(pager: Pager) -> Result<(), MinusError> {
    let mut runmode = minus_core::RUNMODE.lock();
    assert!(runmode.is_uninitialized(), "Failed to set the RUNMODE. This is caused probably bcause another instance of minus is already running");
    *runmode = minus_core::RunMode::Dynamic;
    drop(runmode);
    init::init_core(pager)
}
