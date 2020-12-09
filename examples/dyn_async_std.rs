use async_std::task::sleep;
use futures::join;

use std::fmt::Write;
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = minus::Lines::default();

    let increment = async {
        for i in 0..=30_i32 {
            let mut output = output.lock().unwrap();
            writeln!(output, "{}", i)?;
            drop(output);
            sleep(Duration::from_millis(100)).await;
        }
        Result::<_, std::fmt::Error>::Ok(())
    };

    let (res1, res2) = join!(
        minus::async_std_updating(minus::Lines::clone(&output), minus::LineNumbers::Disabled),
        increment
    );
    res1?;
    res2?;
    Ok(())
}
