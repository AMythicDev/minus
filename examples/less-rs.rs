// This is an example of a pager that uses minus and reads data from a file and pages
// it. It is similar to less, but in Rust. Hence the name `less-rs`
// This example uses OS threads,

// This example uses a lot of `.expect()` and does not properly handle them. If
// someone is interested to add proper error handling, you are free to file pull
//requests. Libraries like anyhow and thiserror are generally preferred and minus
//also uses thiserror

use std::env::args;
use std::fs::File;
use std::io::{BufReader, Read};
use std::thread;

fn read_file(name: String, pager: minus::Pager) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(name)?;
    let changes = || {
        let mut buff = String::new();
        let mut buf_reader = BufReader::new(file);
        buf_reader.read_to_string(&mut buff)?;
        pager.push_str(&buff)?;
        Result::<(), Box<dyn std::error::Error>>::Ok(())
    };

    let pager = pager.clone();
    let res1 = thread::spawn(|| minus::dynamic_paging(pager));
    let res2 = changes();
    res1.join().unwrap()?;
    res2?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let output = minus::Pager::new();
    output.set_prompt(&filename)?;
    read_file(filename, output)
}
