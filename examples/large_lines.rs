use minus::error::MinusError;

fn main() -> Result<(), MinusError> {
    let output = minus::Pager::new();

    for i in 0..30 {
        for _ in 0..=10 {
            output.push_str(&format!("{}. Hello ", i))?;
        }
        output.push_str("\n")?;
    }

    minus::page_all(output)?;
    Ok(())
}
