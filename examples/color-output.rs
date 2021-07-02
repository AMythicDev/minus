use crossterm::style::{Color, ResetColor, SetForegroundColor};
use minus::{error::PageAllError, page_all, Pager};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut pager = Pager::new()?;
    for _ in 1..=30 {
        writeln!(
            pager,
            "{}These are some lines{}",
            SetForegroundColor(Color::Blue),
            ResetColor
        )?;
    }
    page_all(pager)?;
    Ok(())
}
