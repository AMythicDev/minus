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

#[derive(PartialEq, Eq)]
pub enum RunMode {
    #[cfg(feature = "static_output")]
    Static,
    #[cfg(feature = "dynamic_output")]
    Dynamic,
    Uninitialized,
}

impl RunMode {
    pub fn is_uninitialized(&self) -> bool {
        *self == Self::Uninitialized
    }
}
