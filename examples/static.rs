use std::fmt::Write;

fn main() -> minus::Result<(), Box<dyn std::error::Error>> {
    let mut output = String::new();

    for i in 1..=30 {
        writeln!(output, "{}", i)?;
    }

    minus::page_all(&output)?;
    Ok(())
}
