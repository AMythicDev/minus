use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = String::new();

    for i in 0..=100 {
        // Helps check wrapping works correctly.
        writeln!(output, "{} -- {}", i, "=~".repeat(i))?;
    }

    minus::page_all(&output, minus::LineNumbers::Disabled)?;
    Ok(())
}
