use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{prelude::*, stdout};
use std::time::Duration;
use crate::Lines;
use crate::utils::draw;

fn init(mutex: Lines) {
    let _ = execute!(stdout(), EnterAlternateScreen);
    let _ = enable_raw_mode();

    let (_, rows) = crossterm::terminal::size().unwrap();
    let rows = rows as usize;
    let mut upper_mark = 0 as usize;
    let mut last_copy = String::new();

    loop {
        let string = mutex.try_lock();
        if string.is_err() {
            continue;
        }
        let string = string.unwrap();
        if !string.eq(&last_copy) {
            draw(&string, rows, &mut upper_mark.clone());
            last_copy = string.clone();
        }
        drop(string);

        if poll(Duration::from_millis(10)).unwrap() {
            match read().unwrap() {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    modifiers: KeyModifiers::NONE,
                }) | Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL
                }) => {
                    let _ = execute!(stdout(), LeaveAlternateScreen);
                    let _ = disable_raw_mode();
                    std::process::exit(0);
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Down,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    upper_mark += 1;
                    draw(&mutex.lock().unwrap(), rows, &mut upper_mark)
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    if upper_mark != 0 {
                        upper_mark -= 1;
                    }
                    draw(&mutex.lock().unwrap(), rows, &mut upper_mark)
                }
                _ => {}
            }
        }
    }
}

/// Run the pager inside a [`tokio task`](tokio::task)
///
/// This function is only available when `tokio_lib` feature is enabled
/// This function switches to the [`Alternate Screen`] of the TTY and switches to
/// [`raw mode`]
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

/// Run the pager inside a [`async_std task`]
///
/// This function is only available when `async_std_lib` feature is enabled
/// This function switches to the [`Alternate Screen`] of the TTY and switches to
/// [`raw mode`]
///
/// [`async_std task`]: async_std::task
/// [`Alternate Screen`]: ../crossterm/terminal/index.html#alternate-screen
/// [`raw mode`]: ../crossterm/terminal/index.html#raw-mode
#[cfg(feature = "async_std_lib")]
pub async fn async_std_updating(mutex: Lines) {
    use async_std::task;
    task::spawn(async move {
        init(mutex);
    }).await;
}
