use async_std::task::sleep;
use futures::{executor::block_on, join};
use minus::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::fmt::Write;

fn main() {
    let output = Arc::new(Mutex::new(String::new()));
    let increment = async {
        let mut counter: u8 = 0;
        while counter <= 30 {
            let mut output = output.lock().unwrap();
            writeln!(output, "{}", counter.to_string());
            counter += 1;
            drop(output);
            sleep(Duration::from_millis(100)).await;
        }
    };
    let run = async {
        join!(async_std_updating(output.clone()), increment);
    };
    block_on(run);
}
