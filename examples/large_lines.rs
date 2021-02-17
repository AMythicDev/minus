use std::fmt::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = minus::Pager::new();

    for _ in 0..30 {
        for _ in 0..=30 {
            output.push_str("Hello ")
        }
    }

    minus::page_all(output)?;
    Ok(())
}
