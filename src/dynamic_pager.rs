use crate::Pager;
use crate::error::MinusError;
use crate::minus_core::init;

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
#[allow(clippy::needless_pass_by_value)]
pub fn dynamic_paging(pager: Pager) -> Result<(), MinusError> {
    use crate::pager::AliveGuard;
    use std::sync::Arc;
    // Build a new Pager whose `alive` Arc is independent of the one held by
    // application-side clones.  When this local Pager drops (after `init_core`
    // returns) only the independent Arc is decremented, which is harmless.
    //
    // Dropping `pager` here decrements the application-side Arc so that the
    // correct reference count is maintained: only the application-side handles
    // should keep that Arc alive.
    let pager_for_init = Pager {
        tx: pager.tx.clone(),
        rx: pager.rx.clone(),
        // New, independent Arc that fires a CheckQuitIfOneScreen into an already-
        // closed channel once init_core returns – the send error is silently ignored.
        alive: Arc::new(AliveGuard::new(pager.tx.clone())),
    };
    drop(pager); // decrement the application-side Arc (count N+1 → N)
    init::init_core(&pager_for_init, crate::RunMode::Dynamic)
}
