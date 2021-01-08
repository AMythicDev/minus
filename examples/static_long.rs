use std::fmt::Write;
use tracing::{subscriber, Level};
use tracing_appender::{non_blocking, rolling::never};
use tracing_subscriber::fmt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = never("./", "minus.log");
    let (non_block, _guard) = non_blocking(file);
    let subscriber = fmt()
        .with_writer(non_block)
        .with_max_level(Level::INFO)
        .compact()
        .finish();

    subscriber::set_global_default(subscriber).unwrap();

    let mut output = minus::Pager::new();

    for i in 0..=100 {
        writeln!(output.lines, "{}", i)?;
    }

    minus::page_all(output)?;
    Ok(())
}
