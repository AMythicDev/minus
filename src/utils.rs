use std::sync::MutexGuard;
use crossterm::{terminal::{Clear, ClearType}, cursor::MoveTo};
use std::io::prelude::*;
use crossterm::style::Attribute;
use std::fmt::Write;
use std::io::Write as IOWrite;

const LINE_NUMBERS: bool = false;

pub(crate) fn draw(lines: &MutexGuard<String>, rows: usize, upper_mark: &mut usize) {
    // Split the String on each \n
    let lines: Vec<&str> = lines.split_terminator('\n').collect();
    // Calculate the lower mark
    let mut lower_mark = *upper_mark + rows - 1;

    // Do some necessory checking
    // Lower mark should not be more than the lenght of lines vector
    if lower_mark >= lines.len() {
        lower_mark = lines.len();
        // If the length of lines is less than the number of rows, set upper_mark = 0
        *upper_mark = if lines.len() < rows {
            0
        } else {
            // Otherwise, set upper_mark to lenght of lines - rows
            lines.len() - rows
        };
    }

    // Get the range of lines between upper mark and lower mark
    let range = &lines[*upper_mark..lower_mark];
    // Clear the screen and place cursor at the very top left
    print!("{}{}", Clear(ClearType::All), MoveTo(0,0));

    if LINE_NUMBERS == false {
        // Join the range with \n\r
        let format_lines = range.join("\n\r");
        // Write the text, make sure to \r before and after output for
        // correct cursor placement before/after output
        println!("\r{}\r", format_lines);
    } else {
        // Wrtee each line of the output to the String
        let mut output = String::new();
        for (index, line) in range.iter().enumerate() {
            // Put the output to output variable
            writeln!(output, "\r{}. {}\n", *upper_mark + index + 1, line);
        }
        // Output the data
        // Printing each line to terminal can be slow, so write the data to a variable and finally flush it
        print!("{}", output);
        std::io::stdout().flush();
    }

    // Display the prompt
    print!("{}{}Press q or Ctrl+C to quit", MoveTo(0, rows as u16),Attribute::Reverse);
    std::io::stdout().flush();
    print!("{}", Attribute::Reset);
}