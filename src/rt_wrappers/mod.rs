use crate::error::AlternateScreenPagingError;
use crate::init;
use crate::PagerMutex;
use std::sync::Arc;

#[cfg(feature = "async_std_lib")]
pub mod async_std_wrapper;

#[cfg(feature = "tokio_lib")]
pub mod tokio_wrapper;

/// Private function that contains the implemenation for the async display.
async fn run(pager: Arc<PagerMutex>) -> Result<(), AlternateScreenPagingError> {
    init::dynamic_paging(&pager).await
}
