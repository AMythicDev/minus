use std::sync::MutexGuard;
use crossterm::{terminal::{Clear, ClearType}, cursor::MoveTo};
use std::io::prelude::*;
use crossterm::style::Attribute;
use std::fmt::Write;
use std::io::Write as IOWrite;

const LINE_NUMBERS: bool = false;

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
    print!("{}{}", Clear(ClearType::All), MoveTo(0,0));

    if LINE_NUMBERS == false {
        let format_lines = range.join("\n\r");
        println!("\r{}\r", format_lines);
    } else {
        let mut output = String::new();
        for (index, line) in range.iter().enumerate() {
            writeln!(output, "\r{}. {}", *upper_mark + index + 1, line);
        }
        print!("{}", output);
        std::io::stdout().flush();
    }

    print!("{}{}Press q or Ctrl+C to quit", MoveTo(0, rows as u16),Attribute::Reverse);
    std::io::stdout().flush();
    print!("{}", Attribute::Reset);
}