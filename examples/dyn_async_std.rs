use async_std::task::sleep;
use futures::join;

use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Arc::new(Mutex::new(String::new()));

    let increment = async {
        let mut counter: u8 = 0;
        while counter <= 30 {
            let mut output = output.lock().unwrap();
            writeln!(output, "{}", counter.to_string())?;
            counter += 1;
            drop(output);
            sleep(Duration::from_millis(100)).await;
        }
        Result::<_, std::fmt::Error>::Ok(())
    };

    let (res1, res2) = join!(
        minus::async_std_updating(Arc::clone(&output), minus::LineNumbers::Disabled),
        increment
    );
    res1?;
    res2?;
    Ok(())
}
