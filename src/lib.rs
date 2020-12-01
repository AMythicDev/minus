/// TODO: Clean up this section
use crossterm::{
    cursor::{MoveTo, MoveToColumn},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    terminal::{Clear, ClearType, ScrollUp, ScrollDown},
};
use futures::join;
use std::io::{prelude::*, stdout};
use std::sync::{Arc, Mutex, MutexGuard};
use std::sync::mpsc::Receiver;
use std::time::Duration;
use async_std::task::{self, JoinHandle};
use async_std::task::sleep;

type Lines = Arc<Mutex<String>>;

fn draw(lines: &MutexGuard<String>, rows: usize, upper_mark: usize) {
    let lines: Vec<&str> = lines.split_terminator('\n').collect();
    let mut lower_mark = upper_mark + rows;
    if lower_mark >= lines.len() {
        lower_mark = lines.len();
    }
    let range = &lines[upper_mark..lower_mark];

    let format_lines = range.connect("\n\r");
    print!("{}{}", Clear(ClearType::All), MoveTo(0,0));
    println!("\r{}", format_lines);
}

pub async fn refreshable(mutex: Lines) {
    task::spawn(async move {
        let _ = execute!(stdout(), EnterAlternateScreen);
        let _ = enable_raw_mode();

        let (cols, rows) = crossterm::terminal::size().unwrap();
        let rows = rows as usize;
        let upper_mark = 0 as usize;
        let mut last_copy = String::new();

        loop {
            let string = mutex.try_lock();
            if string.is_err() {
                continue;
            }
            let string = string.unwrap();
            if !string.eq(&last_copy) {
                draw(&string, rows, upper_mark);
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
            //         Event::Key(KeyEvent {
            //             code: KeyCode::Down,
            //             modifiers: KeyModifiers::NONE,
            //         }) => {
            //             lower_mark += 1;
            //         }
            //         Event::Key(KeyEvent {
            //             code: KeyCode::Up,
            //             modifiers: KeyModifiers::NONE,
            //         }) => {
            //             lower_mark -= 1;
            //         }
                    _ => {}
                }
            }
        }
    }).await;
}






/*
type Lines = Arc<Mutex<Vec<String>>>;

async fn update_string<T>(cx: Receiver<Signal<T>>, lines: Lines)
where
    T: std::fmt::Display + Into<String> + Clone,
{
    async_std::task::sleep(std::time::Duration::from_millis(50));
    loop {
        let recv = cx.try_recv();
        match recv {
            Ok(Signal::Data(text)) => {
                let borrow = lines.try_lock();

                if borrow.is_none() {
                    async_std::task::sleep(std::time::Duration::from_millis(50)).await;
                    continue;
                }
                let mut borrow = borrow.unwrap();
                let string: String = text.into();
                let buf_line: Vec<&str> = string.split_terminator('\n').collect();
                buf_line.iter().for_each(|item| {
                    borrow.push(item.to_string());
                });
                println!("\r{:?}", borrow);
            }
            Ok(Signal::Close) => {
                break;
            }
            Err(_) => {
                async_std::task::sleep(std::time::Duration::from_millis(50));
            }
        }
    }
}

fn write_all(lines: &MutexGuard<Vec<String>>) {
    print!("{}{}", Clear(ClearType::All), MoveTo(0, 0));
    println!("{:?}", lines);
    // let concat_data = lines.connect("\n\r");
    // println!("\r{}", concat_data);
    // execute!(stdout(), MoveTo(0, (*rows as u16)));
}

async fn draw(lines: Lines) {
    let _ = execute!(stdout(), EnterAlternateScreen);
    let _ = enable_raw_mode();

    let (_, rows) = crossterm::terminal::size().unwrap();
    let rows = rows as usize;
    let mut lower_mark = rows;
    let mut upper_mark = 0;
    let mut last_lower_mark = 0;
    loop {
        if lower_mark != last_lower_mark {
            let borrow = lines.try_lock();

            let borrow = borrow.unwrap();
            let lines_len = borrow.len();
            if lines_len < rows - 1 {
                if borrow.is_empty() {
                    async_std::task::sleep(std::time::Duration::from_millis(50)).await;
                }
                write_all(&borrow);
            } else {
                last_lower_mark = lower_mark;
            }
            
            drop(borrow);
        }
    }
}

pub async fn run<T: 'static>(cx: Receiver<Signal<T>>)
where
    T: std::fmt::Display + Into<String> + Clone + Send,
{
    let lines: Lines = Arc::new(Mutex::new(Vec::new()));
    let lines_copy = lines.clone();
    let update_handle = async_std::task::spawn(async move {
        update_string(cx, lines_copy).await;
    });
    let draw_handle = async_std::task::spawn(async move {
        draw(lines).await;
    });
    join!(update_handle, draw_handle);
}
*/