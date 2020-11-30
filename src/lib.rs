/// TODO: Clean up this section
use async_std::task::sleep;
use crossterm::{
    cursor::{MoveTo, MoveToColumn},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    terminal::{Clear, ClearType, ScrollUp},
};
use futures::join;
use std::cell::RefCell;
use std::io::{prelude::*, stdout};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::time::Duration;

/// Different signals that could be sent to the pager
///
/// When using the pager, there are mostly two signals that you want to
/// send
/// * Some data
/// * The end of all data
/// The signals should be passed with the `send` method of the Sender
/// ## Example
/// TODO
pub enum Signal<T>
where
    T: std::fmt::Display + Into<String> + Clone,
{
    Data(T),
    Close,
}

type Lines = Arc<Mutex<Vec<String>>>;

async fn update_string<T>(cx: Receiver<Signal<T>>, lines: Lines)
where
    T: std::fmt::Display + Into<String> + Clone,
{
    loop {
        match cx.try_recv() {
            Ok(Signal::Data(text)) => {
                let mut lines = lines.lock().unwrap();
                let string: String = text.into();
                let buf_line: Vec<&str> = string.split_terminator('\n').collect();
                buf_line.iter().for_each(|item| {
                    lines.push(item.to_string());
                });
                drop(lines);
            }
            Ok(Signal::Close) => {
                break;
            }
            Err(_) => {}
        }
    }
}

async fn draw(lines: Lines) {
    let _ = execute!(stdout(), EnterAlternateScreen);
    let _ = enable_raw_mode();
    let mut last_printed = 0;

    loop {
        let borrow = lines.lock().unwrap();
        if last_printed < borrow.len() {
            print!("{}{}", Clear(ClearType::All), MoveTo(0, 0),);
            for line in borrow.iter() {
                println!("{}{}", line, MoveToColumn(0));
            }
            last_printed = borrow.len();
        }
        drop(borrow);

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
                    code: KeyCode::Up,
                    modifiers: KeyModifiers::NONE,
                }) => {
                    ScrollUp(1);
                }
                _ => {}
            }
        }
    }
}

pub async fn run<T: 'static>(cx: Receiver<Signal<T>>)
where
    T: std::fmt::Display + Into<String> + Clone + Send,
{
    let lines: Lines = Arc::new(Mutex::new(Vec::new()));
    let draw_handle = async_std::task::spawn(
        draw(lines.clone())
    );
    let update_handle = async_std::task::spawn(
        update_string(cx, lines.clone())
    );
    join!(update_handle, draw_handle);
}
