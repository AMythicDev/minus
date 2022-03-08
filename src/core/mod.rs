#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
mod display;
#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
mod ev_handler;
pub mod events;
#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
pub mod init;
#[cfg(feature = "search")]
pub mod search;
#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
pub mod term;
