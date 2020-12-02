use std::sync::MutexGuard;
use crossterm::{terminal::{Clear, ClearType}, cursor::MoveTo};

pub(crate) fn draw(lines: &MutexGuard<String>, rows: usize, upper_mark: &mut usize) {
    let lines: Vec<&str> = lines.split_terminator('\n').collect();
    let mut lower_mark = *upper_mark + rows - 1;

    if lower_mark >= lines.len() {
        lower_mark = lines.len();
        *upper_mark = if lines.len() < rows {
            0
        } else {
            lines.len() - rows
        };
    }

    let range = &lines[*upper_mark..lower_mark];

    let format_lines = range.join("\n\r");
    print!("{}{}", Clear(ClearType::All), MoveTo(0,0));
    println!("\r{}\r", format_lines);
}