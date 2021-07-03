use futures::join;
use std::time::Duration;
use tokio::time::sleep;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = minus::Pager::new().unwrap().finish();

    let increment = async {
        for i in 0..=100_u32 {
            let mut output = output.lock().await;
            output.push_str(format!("{}\n", i));
            drop(output);
            sleep(Duration::from_millis(100)).await;
        }
        let mut output = output.lock().await;
        output.end_data_stream();
        Result::<_, std::fmt::Error>::Ok(())
    };

    let (res1, res2) = join!(minus::tokio_updating(output.clone()), increment);
    res1?;
    res2?;
    Ok(())
}
