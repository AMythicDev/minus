use futures::join;
use tokio::time::sleep;

use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[tokio::main]
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

    let (res1, res2) = join!(minus::tokio_updating(output.clone()), increment);
    res1?;
    res2?;
    Ok(())
}
