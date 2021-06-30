fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = minus::Pager::new().unwrap();

    for i in 0..=100 {
        output.push_str(format!("{}\n", i))
    }

    minus::page_all(output)?;
    Ok(())
}
