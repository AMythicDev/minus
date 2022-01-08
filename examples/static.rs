use minus::error::MinusError;

fn main() -> Result<(), MinusError> {
    let output = minus::Pager::new();

    for i in 0..=100 {
        output.push_str(&format!("{}\n", i))?;
    }

    minus::page_all(output)?;
    Ok(())
}
