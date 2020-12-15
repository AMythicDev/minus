// This is an example of a pager that uses minus and reads data from a file and pages
// it. It is similar to less, but in Rust. Hence the name `less-rs`
// This example uses async-std runtime, though you can use tokio, or even blocking code
// just make sure to enable the proper feature

// This example uses a lot of `.expect()` and does not properly handle them. If
// someone is interested to add proper error handling, you are free to file pull
//requests. Libraries like anyhow and thiserror are generally prefered and minus
//also uses thiserror

use async_std::io::prelude::*;
use futures::future::join;
use std::env::args;

// async fn read_file(name: String, pager: minus::PagerMutex) -> Result<(), std::io::Error> {
async fn read_file(
    name: String,
    pager: minus::PagerMutex,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = async_std::fs::File::open(name).await?;
    let changes = async {
        let mut buf_reader = async_std::io::BufReader::new(file);
        let mut guard = pager.lock().unwrap();
        buf_reader.read_to_string(&mut guard.lines).await?;
        std::io::Result::<_>::Ok(())
    };

    let (res1, res2) = join(minus::async_std_updating(pager.clone()), changes).await;
    res1?;
    res2?;
    Ok(())
}

#[async_std::main]
// async fn main() {
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get the file name from the command line
    // Typically, you want to use something like clap here, but we are not doing it
    // here to make the example simple
    let arguments: Vec<String> = args().collect();
    // Check if there are is at least two arguments including the program name
    if arguments.len() < 2 {
        // You probably want to do a graceful exit, but we are panicking here to make
        // example short
        panic!("Not enough arguments");
    }
    // Get the filename
    let filename = arguments[1].clone();
    // Initialize the configuration
    let pager = minus::Pager::default_dynamic();
    read_file(filename, pager).await
}
