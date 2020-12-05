use crate::utils::draw;
use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::stdout;
use std::io::Write;

/// Outputs static information
///
///. Once called, string passed to this function can never be changed. If you want
/// dynamic information, see [`async_std_updating`] and [`tokio_updating`]
///
/// [`async_std_updating`]: crate::rt_wrappers::async_std_updating
/// [`tokio_updating`]: crate::rt_wrappers::tokio_updating
///
/// ## Example
/// ```
/// fn main() {
///     let mut output = String::new();
///     for i in 1..=30 {
///         let _ = writeln!(output, "{}", i);
///     }
///     minus::page_all(output);
/// }
/// ```
pub fn page_all(lines: String) {
    // Get terminal rows and convert it to usize
    let (_, rows) = crossterm::terminal::size().unwrap();
    let mut rows = rows as usize;

    // If the number of lines in the output is less than the number of rows
    // then print it and quit
    {
        let range: Vec<&str> = lines.split_terminator("\n").collect();
        if rows > range.len() {
            for line in range {
                println!("{}", line);
            }
            std::process::exit(0);
        }
    }

    // Initialize the terminal
    let _ = execute!(stdout(), EnterAlternateScreen);
    let _ = enable_raw_mode();
    let _ = execute!(stdout(), Hide);

    // The upper mark of scrolling
    let mut upper_mark = 0 as usize;

    // Draw at the very beginning
    draw(lines.clone(), rows, &mut upper_mark);

    loop {
        if poll(std::time::Duration::from_millis(10)).unwrap() {
            match read().unwrap() {
                // If q or Ctrl+C is pressed, reset all changes to the terminal and quit
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                })
                | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }) => {
                    let _ = execute!(stdout(), LeaveAlternateScreen);
                    let _ = disable_raw_mode();
                    let _ = execute!(stdout(), Show);
                    std::process::exit(0);
                }
                // If Down arrow is pressed, add 1 to the marker and update the string
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    upper_mark += 1;
                    draw(lines.clone(), rows, &mut upper_mark)
                }
                // If Up arrow is pressed, subtract 1 from the marker and update the string
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    if upper_mark != 0 {
                        upper_mark -= 1;
                    }
                    draw(lines.clone(), rows, &mut upper_mark)
                }
                // When terminal is resized, update the rows and redraw
                Event::Resize(_, height) => {
                    rows = height as usize;
                    draw(lines.clone(), rows, &mut upper_mark)
                }
                _ => {}
            }
        }
    }
}
