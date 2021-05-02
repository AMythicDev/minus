fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = minus::Pager::new();

    for i in 0..=30 {
        output.push_str(format!("{}\n", i));
    }

    minus::page_all(output)?;
    Ok(())
}
