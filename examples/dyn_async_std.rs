use async_std::task::{sleep, spawn};
use futures_lite::future;
use minus::{async_paging, error::MinusError};
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), MinusError> {
    let output = minus::Pager::new();

    let increment = async {
        for i in 0..=100_u32 {
            output.push_str(&format!("{}\n", i))?;
            sleep(Duration::from_millis(100)).await;
        }
        Result::<_, MinusError>::Ok(())
    };

    let (res1, res2) = future::zip(spawn(async_paging(output.clone())), increment).await;
    res1?;
    res2?;
    Ok(())
}
