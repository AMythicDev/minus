use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = minus::Pager::new();

    for i in 0..=30 {
        writeln!(output.lines, "{}", i)?;
    }

    minus::page_all(output)?;
    Ok(())
}
