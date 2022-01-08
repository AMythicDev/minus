use crossterm::style::{Color, ResetColor, SetForegroundColor};
use minus::{error::MinusError, page_all, Pager};
use std::fmt::Write;

fn main() -> Result<(), MinusError> {
    let mut pager = Pager::new();
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
