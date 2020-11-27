use futures::{executor::block_on, join};
use minus::*;
use std::sync::mpsc::channel;
use std::time::Duration;
use async_std::task::sleep;

fn main() {
    let (tx, cx) = channel();
    let increment = async {
        let mut counter = 0;
        while counter <= 5 {
            tx.send(Signal::Data(counter));
            counter += 1;
            sleep(Duration::from_millis(1000)).await;
        }
        tx.send(Signal::Close);
    };
    let run = async {
        join!(increment, run(cx));
    };
    block_on(run);
}
