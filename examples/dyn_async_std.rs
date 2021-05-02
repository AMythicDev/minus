use async_std::task::sleep;
use futures::join;

use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = minus::Pager::new().finish();

    let increment = async {
        for i in 0..=30_u32 {
            let mut output = output.lock().await;
            output.push_str(format!("{}\n", i));
            drop(output);
            sleep(Duration::from_millis(100)).await;
        }
        Result::<_, std::fmt::Error>::Ok(())
    };

    let (res1, res2) = join!(minus::async_std_updating(output.clone()), increment);
    res1?;
    res2?;
    Ok(())
}
