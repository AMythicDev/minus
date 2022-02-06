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
use crate::{
    error::MinusError,
    events::Event,
    utils::{draw, ev_handler::handle_event, term::setup},
    Pager, PagerState,
};

use crossbeam_channel::Receiver;
#[cfg(any(
    feature = "async_output",
    feature = "static_output",
    feature = "threads_output"
))]
use once_cell::sync::OnceCell;
use std::io::stdout;
use std::io::Stdout;
#[cfg(feature = "search")]
use std::sync::atomic::AtomicBool;
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
#[cfg(any(feature = "static_output", feature = "threads_output"))]
use {crate::input::reader::polling, std::thread};
#[cfg(feature = "async_output")]
use {crate::input::reader::streaming, futures_lite::future};
#[cfg(feature = "static_output")]
use {crate::utils::write_lines, crossterm::tty::IsTty};

//#[cfg(all(feature = "async_output", feature = "static_output", feature = "threads_output"))]

#[cfg(any(
    feature = "async_output",
    feature = "static_output",
    feature = "threads_output"
))]
pub(crate) enum RunMode {
    #[cfg(feature = "static_output")]
    Static,
    #[cfg(feature = "async_output")]
    Async,
    #[cfg(feature = "threads_output")]
    Thread,
}

#[cfg(any(
    feature = "async_output",
    feature = "static_output",
    feature = "threads_output"
))]
pub(crate) static RUNMODE: OnceCell<RunMode> = OnceCell::new();

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
/// Next it initializes the runtime and calls [`start_reactor`] and a event reader which is
/// selected based on the enabled feature set:-
///
/// * If both `static_output` and `async_output` features are selected
///     * If running in static mode, a [polling] based event reader is spawned on a
///     thread and the [`start_reactor`] is called directly
///     * If running in async mode, a [streaming] based event reader and [`start_reactor`] are
///     spawned in a `async_global_allocatior` task
///
/// * If only `static_output` feature is enabled, [polling] based event reader is spawned
/// on a thread and the [`start_reactor`] is called directly
/// * If only `async_output` feature is enabled, [streaming] based event reader and
/// [`start_reactor`] are spawned in a `async_global_allocatior` task
///
/// # Errors
///
/// Setting/cleaning up the terminal can fail and IO to/from the terminal can
/// fail.
///
/// [streaming]: crate::input::reader::streaming
/// [polling]: crate::input::reader::polling
#[cfg(any(
    feature = "async_output",
    feature = "static_output",
    feature = "threads_output"
))]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn init_core(mut pager: Pager) -> std::result::Result<(), MinusError> {
    let mut out = stdout();
    // Is the event reader running
    #[cfg(feature = "search")]
    let input_thread_running = Arc::new(AtomicBool::new(true));
    #[allow(unused_mut)]
    let mut ps = generate_initial_state(&mut pager.rx, &mut out)?;

    // Static mode checks
    #[cfg(feature = "static_output")]
    {
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
    setup(&out)?;

    let ps_mutex = Arc::new(Mutex::new(ps));

    #[cfg(any(feature = "static_output", feature = "threads_output"))]
    let start_no_async = || -> Result<(), MinusError> {
        let evtx = pager.tx.clone();
        let rx = pager.rx.clone();
        let out = stdout();

        let p1 = ps_mutex.clone();

        #[cfg(feature = "search")]
        let input_thread_running = input_thread_running.clone();
        #[cfg(feature = "search")]
        let input_thread_running2 = input_thread_running.clone();

        thread::spawn(move || {
            polling(
                &evtx,
                &p1,
                #[cfg(feature = "search")]
                &input_thread_running2,
            )
        });
        start_reactor(
            &rx,
            &ps_mutex,
            out,
            #[cfg(feature = "search")]
            &input_thread_running,
        )?;
        Ok(())
    };

    #[cfg(feature = "async_output")]
    let start_async = || -> Result<(), MinusError> {
        let evtx = pager.tx.clone();
        let rx = pager.rx.clone();
        let out = stdout();

        let p1 = ps_mutex.clone();
        let p2 = p1.clone();

        #[cfg(feature = "search")]
        let input_thread_running = input_thread_running.clone();
        #[cfg(feature = "search")]
        let input_thread_running2 = input_thread_running.clone();
        let input_reader = async_global_executor::spawn(streaming(
            evtx,
            p2,
            #[cfg(feature = "search")]
            input_thread_running2,
        ));
        let reactor = async_global_executor::spawn_blocking(move || {
            start_reactor(
                &rx,
                &p1,
                out,
                #[cfg(feature = "search")]
                &input_thread_running,
            )
        });
        let task = future::zip(input_reader, reactor);
        let (res1, res2) = async_global_executor::block_on(task);
        res1?;
        res2?;
        Ok(())
    };

    #[allow(clippy::match_same_arms)]
    match RUNMODE.get() {
        #[cfg(feature = "threads_output")]
        Some(&RunMode::Thread) => start_no_async(),
        #[cfg(feature = "static_output")]
        Some(&RunMode::Static) => start_no_async(),
        #[cfg(feature = "async_output")]
        Some(&RunMode::Async) => start_async(),
        None => panic!("RUNMODE not set"),
    }
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
#[cfg(any(
    feature = "async_output",
    feature = "static_output",
    feature = "threads_output"
))]
fn start_reactor(
    rx: &Receiver<Event>,
    ps: &Arc<Mutex<PagerState>>,
    mut out: Stdout,
    #[cfg(feature = "search")] input_thread_running: &Arc<AtomicBool>,
) -> Result<(), MinusError> {
    // Is the terminal completely filled with text
    #[cfg(any(feature = "async_output", feature = "threads_output"))]
    let mut filled = false;
    // Has the user quitted
    let is_exitted: RefCell<bool> = RefCell::new(false);

    {
        let mut p = ps.lock().unwrap();
        draw(&mut out, &mut p)?;
    }
    let out = RefCell::new(out);

    #[cfg(any(feature = "async_output", feature = "threads_output"))]
    let mut dynamic_matcher = || -> Result<(), MinusError> {
        loop {
            if *is_exitted.borrow() {
                break;
            }

            match rx.try_recv() {
                Ok(ev) if ev.required_immidiate_screen_update() => {
                    let mut p = ps.lock().unwrap();
                    handle_event(
                        ev,
                        &mut *out.borrow_mut(),
                        &mut p,
                        &mut is_exitted.borrow_mut(),
                        #[cfg(feature = "search")]
                        input_thread_running,
                    )?;
                    draw(&mut *out.borrow_mut(), &mut p)?;
                }
                Ok(Event::AppendData(text)) => {
                    let mut p = ps.lock().unwrap();
                    handle_event(
                        Event::AppendData(text),
                        &mut *out.borrow_mut(),
                        &mut p,
                        &mut is_exitted.borrow_mut(),
                        #[cfg(feature = "search")]
                        input_thread_running,
                    )?;
                    if p.num_lines() > p.rows {
                        // Check if the terminal just got filled
                        // If so, fill any unfilled row towards the end of the screen
                        if !filled || p.message.1 {
                            draw(&mut *out.borrow_mut(), &mut p)?;
                            filled = true;
                            if p.message.1 {
                                p.message.1 = false;
                            }
                        }
                    }
                    // Immidiately append data to the terminal until we haven't overflowed
                    if p.num_lines() < p.rows || p.message.1 {
                        draw(&mut *out.borrow_mut(), &mut p)?;
                        if p.message.1 {
                            p.message.1 = false;
                        }
                    }
                }
                Ok(ev) => {
                    let mut p = ps.lock().unwrap();
                    handle_event(
                        ev,
                        &mut *out.borrow_mut(),
                        &mut p,
                        &mut is_exitted.borrow_mut(),
                        #[cfg(feature = "search")]
                        input_thread_running,
                    )?;
                }
                Err(_) => {}
            }
        }
        Ok(())
    };

    #[cfg(feature = "static_output")]
    let static_matcher = || -> Result<(), MinusError> {
        loop {
            if *is_exitted.borrow() {
                break;
            }

            if let Ok(Event::UserInput(inp)) = rx.try_recv() {
                let mut p = ps.lock().unwrap();
                handle_event(
                    Event::UserInput(inp),
                    &mut *out.borrow_mut(),
                    &mut p,
                    &mut is_exitted.borrow_mut(),
                    #[cfg(feature = "search")]
                    input_thread_running,
                )?;
                draw(&mut *out.borrow_mut(), &mut p)?;
            }        }
        Ok(())
    };

    #[allow(clippy::match_same_arms)]
    match RUNMODE.get() {
        #[cfg(feature = "async_output")]
        Some(&RunMode::Async) => dynamic_matcher()?,
        #[cfg(feature = "threads_output")]
        Some(&RunMode::Thread) => dynamic_matcher()?,
        #[cfg(feature = "static_output")]
        Some(&RunMode::Static) => static_matcher()?,
        None => panic!("RUNMODE not set"),
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
#[cfg(any(
    feature = "async_output",
    feature = "static_output",
    feature = "threads_output"
))]
fn generate_initial_state(
    rx: &mut Receiver<Event>,
    mut out: &mut Stdout,
) -> Result<PagerState, MinusError> {
    let mut ps = PagerState::new()?;
    #[cfg(feature = "search")]
    let input_thread_running = Arc::new(AtomicBool::new(true));
    rx.try_iter().try_for_each(|ev| {
        handle_event(
            ev,
            &mut out,
            &mut ps,
            &mut false,
            #[cfg(feature = "search")]
            &input_thread_running,
        )
    })?;
    Ok(ps)
}
