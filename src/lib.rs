/// TODO: Clean up this section
use async_std::task::sleep;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use termion::screen::AlternateScreen;
use termion::raw::IntoRawMode;
use std::io::{stdout, prelude::*};
use termion::{input::TermRead, event::Key};
use termion::async_stdin;
use termion::cursor::Left;
use termion::clear::All;

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
    T: std::fmt::Display,
{
    Data(T),
    Close,
}

pub async fn run<T>(cx: Receiver<Signal<T>>)
where
    T: std::fmt::Display,
{
    let stdout = stdout().into_raw_mode().unwrap();
    let mut screen = AlternateScreen::from(stdout);
    let mut check = true;
    let mut keys = async_stdin().keys();
    loop {
        if check {
            match cx.try_iter().peekable().peek() {
                Some(Signal::Data(text)) => {
                    write!(screen, "{}", text);
                    writeln!(screen, "{}", Left(1));
                }
                Some(Signal::Close) => {
                    check = false;
                }
                None => {
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
        match keys.next() {
            Some(Ok(Key::Char('q'))) =>{
                drop(screen);
                std::process::exit(0);  
            },
            Some(Ok(_)) => {},
            Some(Err(_)) => println!("Invalid Key"),
            None => {
                continue;
            }
        }
    }
}