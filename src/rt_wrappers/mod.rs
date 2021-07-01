//! Dynamic information within a pager window.
//!
//! See [`tokio_updating`] and [`async_std_updating`] for more information.

use crate::error::AlternateScreenPagingError;
use crate::init;
use crate::PagerMutex;

#[cfg(feature = "async_std_lib")]
pub mod async_std_wrapper;
#[cfg(feature = "async_std_lib")]
pub use async_std_wrapper::async_std_updating;

#[cfg(feature = "tokio_lib")]
pub mod tokio_wrapper;
#[cfg(feature = "tokio_lib")]
pub use tokio_wrapper::tokio_updating;

/// Private function that contains the implemenation for the async display.
async fn run(pager: PagerMutex) -> Result<(), AlternateScreenPagingError> {
    init::dynamic_paging(&pager).await
}
