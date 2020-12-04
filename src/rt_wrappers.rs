use crate::utils::draw;
use crate::Lines;
use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{prelude::*, stdout};
use std::time::Duration;

fn init(mutex: Lines) {
    // Initialize the terminal
    let _ = execute!(stdout(), EnterAlternateScreen);
    let _ = enable_raw_mode();
    let _ = execute!(stdout(), Hide);

    // Get terminal rows and convert it to usize
    let (_, rows) = crossterm::terminal::size().unwrap();
    let mut rows = rows as usize;
    // The upper mark of scrolling
    let mut upper_mark = 0 as usize;
    // Copy of the last displayed string
    // Only needed when there is less data then the number of rows
    let mut last_copy = String::new();

    loop {
        // Lock the data and check errors
        let string = mutex.try_lock();
        if string.is_err() {
            continue;
        }
        // If no errors, compare it with the last displayed string
        // If they are not equal, display the new data
        let string = string.unwrap();
        // Use .eq() here as == cannot compare MutexGuard with a normal string
        if !string.eq(&last_copy) {
            draw(string.clone(), rows, &mut upper_mark.clone());
            // Update the last copy, cloning here becaue string is inside MutexGuard
            last_copy = string.clone();
        }
        // Drop the string
        drop(string);

        // Poll for keypresses
        if poll(Duration::from_millis(10)).unwrap() {
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
                    draw(mutex.lock().unwrap().clone(), rows, &mut upper_mark)
                }
                // If Up arrow is pressed, subtract 1 from the marker and update the string
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    if upper_mark != 0 {
                        upper_mark -= 1;
                    }
                    draw(mutex.lock().unwrap().clone(), rows, &mut upper_mark)
                }
                // When terminal is resized, update the rows and redraw
                Event::Resize(_, height) => {
                    rows = height as usize;
                    draw(mutex.lock().unwrap().clone(), rows, &mut upper_mark)
                }
                _ => {}
            }
        }
    }
}

/// Run the pager inside a [`tokio task`](tokio::task)
///
/// This function is only available when `tokio_lib` feature is enabled
/// It takes a [`Lines`] and updates the page with new information when Lines
/// is updated
///
/// This function switches to the [`Alternate Screen`] of the TTY and
/// switches to [`raw mode`]
/// ## Example
/// ```
/// use std::sync::{Arc, Mutex};
/// use futures::join;
/// use std::fmt::Write;
/// use std::time::Duration;
/// use tokio::time::sleep;
///
/// #[tokio::main]
/// async fn main() {
///     let output = Arc::new(Mutex::new(String::new()));
///     let push_data = async {
///         for i in 1..=100 {
///             let mut guard = output.lock().unwrap();
///             // Always use writeln to add a \n after the line
///             writeln!(guard, "{}", i);
///             // If you have furthur asynchronous blocking code, drop the borrow here
///             drop(guard);
///             // Some asynchronous blocking code
///             sleep(Duration::new(1,0)).await;
///         }
///    };
///    join!(minus::tokio_updating(output.clone()), push_data);
/// }
/// ```
/// **Please do note that you should never lock the output data, since this will cause
/// the paging thread to be paused. Only borrow it when it is required and drop it
/// if you have furthur asynchronous blocking code**
///
/// [`Alternate Screen`]: ../crossterm/terminal/index.html#alternate-screen
/// [`raw mode`]: ../crossterm/terminal/index.html#raw-mode
#[cfg(feature = "tokio_lib")]
pub async fn tokio_updating(mutex: Lines) {
    use tokio::task;
    task::spawn(async move {
        init(mutex);
    });
}

/// Initialize a updating pager inside a [`async_std task`]
///
/// This function is only available when `async_std_lib` feature is enabled
/// It takes a [`Lines`] and updates the page with new information when Lines
/// is updated
/// This function switches to the [`Alternate Screen`] of the TTY and
/// switches to [`raw mode`]
///
/// ## Example
/// ```
/// use std::sync::{Arc, Mutex};
/// use futures::join;
/// use std::time::Duration;
///
/// #[async_std::main]
/// async fn main() {
///     let output = Arc::new(Mutex::new(String::new()));
///     let push_data = async {
///         for i in 1..=100 {
///             let mut guard = output.lock().unwrap();
///             guard.push_str(&i.to_string());
///             // If you have furthur asynchronous blocking code, drop the borrow here
///             drop(guard);
///             // Some asynchronous blocking code
///             async_std::task::sleep(Duration::new(1,0)).await;
///         }
///    };
///    join!(minus::async_std_updating(output.clone()), push_data);
/// }
/// ```
/// **Please do note that you should never lock the output data, since this will cause
/// the paging thread to be paused. Only borrow it when it is required and drop it
/// if you have furthur asynchronous blocking code**
///
/// [`async_std task`]: async_std::task
/// [`Alternate Screen`]: ../crossterm/terminal/index.html#alternate-screen
/// [`raw mode`]: ../crossterm/terminal/index.html#raw-mode
/// [`Lines`]: Lines
#[cfg(feature = "async_std_lib")]
pub async fn async_std_updating(mutex: Lines) {
    use async_std::task;
    task::spawn(async move {
        init(mutex);
    })
    .await;
}
