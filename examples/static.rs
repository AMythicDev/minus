fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = minus::Pager::new();

    for _ in 0..30 {
        for _ in 0..20 {
            output.push_str("Hello ");
        }
        output.push_str('\n')
    }
    // println!("{:?}", output.lines());
    minus::page_all(output)?;
    Ok(())
}
