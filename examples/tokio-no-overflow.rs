use futures::join;
use minus::{tokio_updating, Pager};
use std::fmt::Write;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pager = Pager::new()?;
    pager.set_run_no_overflow(true);
    let pager = pager.finish();
    let incrementor = async {
        for i in 0..=10u32 {
            let mut output = pager.lock().await;
            writeln!(output, "{}", i)?;
            drop(output);
            sleep(Duration::from_millis(200)).await;
        }
        let mut output = pager.lock().await;
        output.end_data_stream();
        Result::<_, std::fmt::Error>::Ok(())
    };
    let (res1, res2) = join!(tokio_updating(pager.clone()), incrementor);
    res1?;
    res2?;
    Ok(())
}
