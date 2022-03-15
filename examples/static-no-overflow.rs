use minus::error::MinusError;
use minus::{page_all, Pager};
use std::fmt::Write;

fn main() -> Result<(), MinusError> {
    let mut pager = Pager::new();
    pager.set_run_no_overflow(true)?;
    for i in 0..=10u32 {
        writeln!(pager, "{}", i)?;
    }
    page_all(pager)?;
    Ok(())
}
