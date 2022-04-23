//! Contains functions that initialize minus
//!
//! This module provides two main functions:-
//! * The [`init_core`] function which is responsible for setting the initial state of the
//! Pager, do enviroment checks and initializing various core functions on either async
//! tasks or native threads depending on the feature set
//!
//! * The [`start_reactor`] function displays the displays the output and also polls
//! the [`Receiver`] held inside the [`Pager`] for events. Whenever a event is
//! detected, it reacts to it accordingly.
use super::{display::draw, ev_handler::handle_event, events::Event, term};
use crate::{error::MinusError, input::InputEvent, Pager, PagerState};

use crossbeam_channel::{Receiver, Sender, TrySendError};
use crossterm::event;
#[cfg(feature = "dynamic_output")]
use crossterm::{terminal::{Clear, ClearType}, execute};
use once_cell::sync::OnceCell;
use std::io::{stdout, Stdout};
use std::sync::{Arc, Mutex};
use std::thread;
#[cfg(feature = "static_output")]
use {super::display::write_lines, crossterm::tty::IsTty};

#[derive(PartialEq)]
pub enum RunMode {
    #[cfg(feature = "static_output")]
    Static,
    #[cfg(feature = "dynamic_output")]
    Dynamic,
}

pub static RUNMODE: OnceCell<RunMode> = OnceCell::new();

/// The main entry point of minus
///
/// This is called by both [`async_paging`](crate::async_paging) and
/// [`page_all`](crate::page_all) functions.
///
/// It first receives all events present inside the [`Pager`]'s receiver
/// and creates the initial state that to be stored inside the [`PagerState`]
///
/// Then it checks if the minus is running in static mode and does some checks:-
/// * If standard output is not a terminal screen, that is if it is a file or block
/// device, minus will write all the data at once to the stdout and quit
///
/// * If the size of the data is less than the available number of rows in the terminal
/// then it displays everything on the main stdout screen at once and quits. This
/// behaviour can be turned off if [`Pager::set_run_no_overflow(true)`] is called
/// by the main application
// Sorry... this behaviour would have been cool to have in async mode, just think about it!!! Many
// implementations were proposed but none were perfect
// It is because implementing this especially with line wrapping and terminal scrolling
// is a a nightmare because terminals are really naughty and more when you have to fight with it
// using your library... your only weapon
// So we just don't take any more proposals about this. It is really frustating to
// to throughly test each implementation and fix out all rough edges around it
/// Next it initializes the runtime and calls [`start_reactor`] and a [`event reader`]` which is
/// selected based on the enabled feature set:-
///
/// # Errors
///
/// Setting/cleaning up the terminal can fail and IO to/from the terminal can
/// fail.
///
/// [`event reader`]: event_reader
#[allow(clippy::module_name_repetitions)]
pub fn init_core(mut pager: Pager) -> std::result::Result<(), MinusError> {
    let mut out = stdout();
    // Is the event reader running
    #[cfg(feature = "search")]
    let input_thread_running = Arc::new(Mutex::new(()));
    #[allow(unused_mut)]
    let mut ps = generate_initial_state(&mut pager.rx, &mut out)?;

    // Static mode checks
    #[cfg(feature = "static_output")]
    if RUNMODE.get() == Some(&RunMode::Static) {
        // If stdout is not a tty, write everyhting and quit
        if !out.is_tty() {
            write_lines(&mut out, &mut ps)?;
            return Ok(());
        }
        // If number of lines of text is less than available wors, write everything and quit
        // unless run_no_overflow is set to true
        if ps.num_lines() <= ps.rows && ps.run_no_overflow {
            write_lines(&mut out, &mut ps)?;
            ps.exit();
            return Ok(());
        }
    }

    // Setup terminal, adjust line wraps and get rows
    term::setup(&out)?;

    let ps_mutex = Arc::new(Mutex::new(ps));

    let evtx = pager.tx.clone();
    let rx = pager.rx.clone();
    let out = stdout();

    let p1 = ps_mutex.clone();

    #[cfg(feature = "search")]
    let input_thread_running2 = input_thread_running.clone();

    thread::spawn(move || {
        event_reader(
            &evtx,
            &p1,
            #[cfg(feature = "search")]
            &input_thread_running2,
        )
    });
    start_reactor(
        &rx,
        &ps_mutex,
        &out,
        #[cfg(feature = "search")]
        &input_thread_running,
    )?;
    Ok(())
}

/// Continously displays the output and reacts to events
///
/// This function displays the output continously while also checking for user inputs.
///
/// Whenever a event like a user input or instruction from the main application is detected
/// it will call [`handle_event`] to take required action for the event.
/// Then it will be do some checks if it is really necessory to redraw the screen
/// and redraw if it event requires it to do so.
///
/// For example if all rows in a terminal aren't filled and a
/// [`AppendData`](crate::events::Event::AppendData) event occurs, it is absolutely necessory
/// to update the screen immidiately; while if all rows are filled, we can omit to redraw the
/// screen.
#[allow(clippy::too_many_lines)]
fn start_reactor(
    rx: &Receiver<Event>,
    ps: &Arc<Mutex<PagerState>>,
    out: &Stdout,
    #[cfg(feature = "search")] input_thread_running: &Arc<Mutex<()>>,
) -> Result<(), MinusError> {
    // Has the user quitted
    let mut is_exitted: bool = false;
    let mut out_lock = out.lock();

    if let Ok(mut p) = ps.lock() {
        draw(&mut out_lock, &mut p)?;
    }

    #[allow(clippy::match_same_arms)]
    match RUNMODE.get() {
        #[cfg(feature = "dynamic_output")]
        Some(&RunMode::Dynamic) => loop {
            use std::{convert::TryInto, io::Write};
            if is_exitted {
                break;
            }

            let event = rx.recv();

            #[allow(clippy::unnested_or_patterns)]
            match event {
                Ok(ev) if ev.required_immidiate_screen_update() => {
                    let mut p = ps.lock().unwrap();
                    handle_event(
                        ev,
                        &mut out_lock,
                        &mut p,
                        &mut is_exitted,
                        #[cfg(feature = "search")]
                        input_thread_running,
                    )?;
                    draw(&mut out_lock, &mut p)?;
                }
                Ok(Event::SetPrompt(ref text)) | Ok(Event::SendMessage(ref text)) => {
                    let mut p = ps.lock().unwrap();
                    if let Ok(Event::SetPrompt(_)) = event {
                        p.prompt = text.to_string();
                    } else {
                        p.message = Some(text.to_string());
                    }
                    p.format_prompt();
                    term::move_cursor(&mut out_lock, 0, p.rows.try_into().unwrap(), false)?;
                    super::display::write_prompt(
                        &mut out_lock,
                        &p.displayed_prompt,
                        p.rows.try_into().unwrap(),
                    )?;
                }
                Ok(Event::AppendData(text)) => {
                    let mut p = ps.lock().unwrap();
                    // Make the string that nneds to be appended
                    let (fmt_text, num_unterminated) = p.make_append_str(&text);

                    if p.num_lines() < p.rows {
                        // Move the cursor to the very next line after the last displayed line
                        term::move_cursor(
                            &mut out_lock,
                            0,
                            p.num_lines()
                                .saturating_sub(p.unterminated)
                                .try_into()
                                .unwrap(),
                            false,
                        )?;
                        execute!(out_lock, Clear(ClearType::CurrentLine))?;
                        // available_rows -> Rows that are still unfilled
                        //      rows - number of lines displayed -1 (for prompt)
                        // For example if 20 rows are in total in a terminal
                        // and 10 rows are already occupied, then this will be equal to 9
                        let available_rows = p.rows.saturating_sub(
                            p.num_lines()
                                .saturating_sub(p.unterminated)
                                .saturating_add(1),
                        );
                        // Minimum amount of text that an be appended
                        // If available_rows is less, than this will be available rows else it will be
                        // the length of the formatted text
                        //
                        // If number of rows in terminal is 23 with 20 rows filled and another 5 lines are given
                        // This woll be equal to 3 as available rows will be 3
                        // If in the above example only 2 lines are needed to be added, this will be equal to 2
                        let num_appendable = fmt_text.len().min(available_rows);
                        write!(out_lock, "{}", fmt_text[0..num_appendable].join("\n\r"))?;
                        out_lock.flush()?;
                    }
                    // Append the formatted string to PagerState::formatted_lines vec
                    p.append_str_on_unterminated(fmt_text, num_unterminated);
                }
                Ok(ev) => {
                    let mut p = ps.lock().unwrap();
                    handle_event(
                        ev,
                        &mut out_lock,
                        &mut p,
                        &mut is_exitted,
                        #[cfg(feature = "search")]
                        input_thread_running,
                    )?;
                }
                Err(_) => {}
            }
        },
        #[cfg(feature = "static_output")]
        Some(&RunMode::Static) => loop {
            if is_exitted {
                break;
            }

            if let Ok(Event::UserInput(inp)) = rx.recv() {
                let mut p = ps.lock().unwrap();
                handle_event(
                    Event::UserInput(inp),
                    &mut out_lock,
                    &mut p,
                    &mut is_exitted,
                    #[cfg(feature = "search")]
                    input_thread_running,
                )?;
                draw(&mut out_lock, &mut p)?;
            }
        },
        None => panic!("Static variable RUNMODE not set"),
    }
    Ok(())
}

/// Generate the initial [`PagerState`]
///
/// This function creates a default [`PagerState`] and fetches all events present in the receiver
/// to create the initial state. This is done before starting the pager so that we
/// can make the optimizationss that are present in static pager mode.
///
/// # Errors
///  This function will return an error if it could not create the default [`PagerState`]or fails
///  to process the events
fn generate_initial_state(
    rx: &mut Receiver<Event>,
    mut out: &mut Stdout,
) -> Result<PagerState, MinusError> {
    let mut ps = PagerState::new()?;
    rx.try_iter().try_for_each(|ev| -> Result<(), MinusError> {
        handle_event(
            ev,
            &mut out,
            &mut ps,
            &mut false,
            #[cfg(feature = "search")]
            &Arc::new(Mutex::new(())),
        )
    })?;
    Ok(ps)
}

fn event_reader(
    evtx: &Sender<Event>,
    ps: &Arc<Mutex<PagerState>>,
    #[cfg(feature = "search")] input_thread_running: &Arc<Mutex<()>>,
) -> Result<(), MinusError> {
    loop {
        #[cfg(feature = "search")]
        let ilock = input_thread_running.lock().unwrap();
        #[cfg(feature = "search")]
        drop(ilock);

        if event::poll(std::time::Duration::from_millis(100))
            .map_err(|e| MinusError::HandleEvent(e.into()))?
        {
            let ev = event::read().map_err(|e| MinusError::HandleEvent(e.into()))?;
            let mut guard = ps.lock().unwrap();
            // Get the events
            let input = guard.input_classifier.classify_input(ev, &guard);
            if let Some(iev) = input {
                if let InputEvent::Number(n) = iev {
                    guard.prefix_num.push(n);
                    guard.format_prompt();
                } else if !guard.prefix_num.is_empty() {
                    guard.prefix_num.clear();
                    guard.format_prompt();
                }
                if let Err(TrySendError::Disconnected(_)) = evtx.try_send(Event::UserInput(iev)) {
                    break;
                }
            } else if !guard.prefix_num.is_empty() {
                guard.prefix_num.clear();
                guard.format_prompt();
            }
        }
    }
    Result::<(), MinusError>::Ok(())
}
