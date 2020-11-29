use async_std::task::sleep;
use futures::{executor::block_on, join};
use minus::*;
use std::sync::mpsc::channel;
use std::time::Duration;

fn main() {
    let (tx, cx) = channel();
    let increment = async {
        let mut counter = 0;
        while counter <= 100 {
            let _ = tx.send(Signal::Data(counter.to_string()));
            counter += 1;
            sleep(Duration::from_millis(100)).await;
        }
        let _ = tx.send(Signal::Close);
    };
    let run = async {
        join!(increment, run(cx));
    };
    block_on(run);
}
