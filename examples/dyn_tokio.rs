use minus::error::MinusError;
use std::fmt::Write;
use std::time::Duration;
use tokio::{join, task::spawn_blocking, time::sleep};

#[tokio::main]
async fn main() -> Result<(), MinusError> {
    let output = minus::Pager::new();

    let increment = async {
        let mut output = output.clone();
        for i in 0..=100_u32 {
            writeln!(output, "{}", i)?;
            sleep(Duration::from_millis(100)).await;
        }
        Result::<_, MinusError>::Ok(())
    };

    let output = output.clone();
    let (res1, res2) = join!(spawn_blocking(move || minus::dynamic_paging(output)), increment);
    res1.unwrap()?;
    res2?;
    Ok(())
}
