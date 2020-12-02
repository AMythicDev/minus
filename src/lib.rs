mod utils;
use std::sync::{Arc, Mutex};
mod rt_wrappers;

/// An atomically reference counted string of all output for the pager
pub type Lines = Arc<Mutex<String>>;

#[cfg(feature = "tokio_lib")]
pub use rt_wrappers::tokio_refreshable;

#[cfg(feature = "async_std_lib")]
pub use rt_wrappers::async_std_refreshable;