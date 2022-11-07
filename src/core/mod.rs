mod display;
pub mod ev_handler;
pub mod events;
#[cfg(any(feature = "dynamic_output", feature = "static_output"))]
pub mod init;
#[cfg(feature = "search")]
pub mod search;
pub mod term;

#[derive(Copy, Clone, PartialEq, Eq)]
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
