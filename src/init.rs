// There maybe a lot of junk that may get imported if not needed. So we allow
// unused imports
#![allow(unused_imports)]

#[cfg(feature = "search")]
use crate::search::{self, SearchMode};

use crate::{
    error::AlternateScreenPagingError,
    input::InputEvent,
    utils::{
        draw, ev_handler,
        term::{cleanup, setup},
    },
    Pager,
};

#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use async_mutex::Mutex;
use crossterm::{cursor::MoveTo, event, execute};
#[cfg(feature = "search")]
use std::convert::{TryFrom, TryInto};
use std::io::{self, Write as _};
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use std::sync::Arc;

// Runs the pager in dynamic mode for the `PagerMutex`.
//
// ## Errors
//
// Setting/cleaning up the terminal can fail and IO to/from the terminal can
// fail.
#[cfg(any(feature = "async_std_lib", feature = "tokio_lib"))]
pub(crate) async fn dynamic_paging(
    p: &Arc<Mutex<Pager>>,
) -> std::result::Result<(), AlternateScreenPagingError> {
    // Setup terminal, adjust line wraps and get rows
    let mut out = io::stdout();
    let guard = p.lock().await;
    let run_no_overflow = guard.run_no_overflow;
    drop(guard);
    setup(&out, true, run_no_overflow)?;
    // Search related variables
    // A marker of which element of s_co we are currently at
    #[cfg(feature = "search")]
    let mut s_mark = 0;
    // Whether to redraw the console
    #[allow(unused_assignments)]
    let mut redraw = true;
    let mut last_line_count = 0;
    let mut is_exitted = false;

    loop {
        if is_exitted {
            return Ok(());
        }
        // Get the lock, clone it and immidiately drop the lock
        let mut guard = p.lock().await;

        // Display the text continously if last displayed line count is not same and
        // all rows are not filled
        let line_count = guard.num_lines();
        let have_just_overflowed = (last_line_count < guard.rows) && (line_count >= guard.rows);
        if have_just_overflowed && !run_no_overflow {
            setup(&out, true, true)?;
        }
        if last_line_count != line_count && (line_count < guard.rows || have_just_overflowed)
            || guard.message.1
        {
            draw(&mut out, &mut guard)?;
            if guard.message.1 {
                guard.message.1 = false;
            }
            last_line_count = line_count;
        }

        if guard.end_stream && !run_no_overflow && line_count <= guard.rows {
            guard.exit();
            return Ok(cleanup(out, &guard.exit_strategy, false)?);
        }

        drop(guard);
        // Check for events
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            // Lock the value again
            let mut lock = p.lock().await;

            // Get the events
            let input = lock.input_classifier.classify_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                lock.upper_mark,
                #[cfg(feature = "search")]
                lock.search_mode,
                lock.line_numbers,
                lock.message.0.is_some(),
                lock.rows,
            );
            ev_handler::handle_input(
                &input,
                &mut lock,
                &mut out,
                &mut redraw,
                #[cfg(feature = "search")]
                &mut s_mark,
                &mut is_exitted,
            )?;
            // If redraw is true, then redraw the screen
            if redraw {
                draw(&mut out, &mut lock)?;
            }
        }
    }
}

// Runs the pager in dynamic mode for the `Pager`.
//
// ## Errors
//
// Setting/cleaning up the terminal can fail and IO to/from the terminal can
// fail.
#[cfg(feature = "static_output")]
pub(crate) fn static_paging(mut pager: Pager) -> Result<(), AlternateScreenPagingError> {
    let mut out = io::stdout();
    setup(&out, false, pager.run_no_overflow)?;
    #[allow(unused_assignments)]
    let mut redraw = true;

    #[cfg(feature = "search")]
    let mut s_mark: usize = 0;
    let mut is_exitted = false;

    draw(&mut out, &mut pager)?;

    loop {
        if is_exitted {
            return Ok(());
        }
        // Check for events
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            // Get the event
            let input = pager.input_classifier.classify_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                pager.upper_mark,
                #[cfg(feature = "search")]
                pager.search_mode,
                pager.line_numbers,
                pager.message.0.is_some(),
                pager.rows,
            );
            // Handle the event
            ev_handler::handle_input(
                &input,
                &mut pager,
                &mut out,
                &mut redraw,
                #[cfg(feature = "search")]
                &mut s_mark,
                &mut is_exitted,
            )?;

            // If there is some input, or messages and redraw is true
            // Redraw the screen
            if (input.is_some() || pager.message.1) && redraw {
                draw(&mut out, &mut pager)?;
            }
        }
    }
}
