// There maybe a lot of junk that may get imported if not needed. So we allow
// unused imports
#![allow(unused_imports)]

#[cfg(feature = "search")]
use crate::search::{self, SearchMode};

use crate::{
    error::AlternateScreenPagingError,
    input::InputEvent,
    utils::{
        draw,
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

#[cfg(feature = "static_output")]
#[allow(clippy::too_many_lines)]
pub(crate) fn static_paging(mut pager: Pager) -> Result<(), AlternateScreenPagingError> {
    let mut out = io::stdout();
    setup(&out, false, !pager.run_no_overflow)?;
    #[allow(unused_assignments)]
    let mut redraw = true;

    #[cfg(feature = "search")]
    let mut s_mark: usize = 0;

    draw(&mut out, &mut pager)?;

    loop {
        // Check for events
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            // Get the events
            let input = pager.input_handler.handle_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                pager.upper_mark,
                #[cfg(feature = "search")]
                pager.search_mode,
                pager.line_numbers,
                pager.rows,
            );
            // Update any data that may have changed
            #[allow(clippy::match_same_arms)]
            match input {
                Some(InputEvent::Exit) => {
                    pager.exit();
                    return Ok(cleanup(out, &pager.exit_strategy, true)?);
                }
                Some(InputEvent::UpdateTermArea(c, r)) => {
                    pager.rows = r;
                    pager.cols = c;
                    pager.readjust_wraps();
                    redraw = true;
                }
                Some(InputEvent::UpdateUpperMark(um)) => {
                    pager.upper_mark = um;
                    redraw = true;
                }
                Some(InputEvent::UpdateLineNumber(l)) => {
                    pager.line_numbers = l;
                    redraw = true;
                }
                // These are same as their dynamic counterparts, except for the fact
                // that they use a blocking fetch_input function
                #[cfg(feature = "search")]
                Some(InputEvent::Search(m)) => {
                    pager.search_mode = m;
                    let string =
                        search::fetch_input_blocking(&mut out, pager.search_mode, pager.rows)?;
                    if !string.is_empty() {
                        pager.search_term = Some(regex::Regex::new(&string)?);
                        search::highlight_search(&mut pager);
                        dbg!(&pager.upper_mark);
                    }
                    search::next_match(&mut pager, &mut s_mark);
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(InputEvent::NextMatch) if pager.search_term.is_some() => {
                    if s_mark < pager.search_idx.len().saturating_sub(1)
                        && pager.upper_mark + pager.rows < pager.num_lines()
                    {
                        s_mark += 1;
                    }
                    if pager.search_idx.len() > s_mark {
                        search::next_match(&mut pager, &mut s_mark);
                        redraw = true;
                    }
                }
                #[cfg(feature = "search")]
                Some(InputEvent::PrevMatch) if pager.search_term.is_some() => {
                    if pager.search_idx.is_empty() {
                        continue;
                    }
                    s_mark = s_mark.saturating_sub(1);
                    // Do the same steps that we have did in NextMatch block
                    let y = pager.search_idx[s_mark];
                    if usize::from(y) <= pager.upper_mark {
                        pager.upper_mark = y.into();
                    }
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(_) => continue,
                None => continue,
            }
            if redraw {
                draw(&mut out, &mut pager)?;
            }
        }
    }
}

/// Runs the pager in dynamic mode for the `PagerMutex`.
///
/// ## Errors
///
/// Setting/cleaning up the terminal can fail and IO to/from the terminal can
/// fail.
#[cfg(any(feature = "async_std_lib", feature = "tokio_lib"))]
#[allow(clippy::too_many_lines)]
pub(crate) async fn dynamic_paging(
    p: &Arc<Mutex<Pager>>,
) -> std::result::Result<(), AlternateScreenPagingError> {
    // Setup terminal, adjust line wraps and get rows
    let mut out = io::stdout();
    let guard = p.lock().await;
    let run_no_overflow = guard.run_no_overflow;
    setup(&out, true, !run_no_overflow)?;
    drop(guard);
    // Search related variables

    // A marker of which element of s_co we are currently at
    #[cfg(feature = "search")]
    let mut s_mark = 0;
    // Whether to redraw the console
    #[allow(unused_assignments)]
    let mut redraw = true;
    let mut last_line_count = 0;

    loop {
        // Get the lock, clone it and immidiately drop the lock
        let mut guard = p.lock().await;

        // Display the text continously if last displayed line count is not same and
        // all rows are not filled
        let line_count = guard.num_lines();
        let have_just_overflowed = (last_line_count < guard.rows) && (line_count >= guard.rows);
        if have_just_overflowed && run_no_overflow {
            setup(&out, true, true)?;
        }
        if last_line_count != line_count && (line_count < guard.rows || have_just_overflowed) {
            draw(&mut out, &mut guard)?;
            last_line_count = line_count;
        }

        if guard.end_stream && run_no_overflow && line_count <= guard.rows {
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
            let input = lock.input_handler.handle_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                lock.upper_mark,
                #[cfg(feature = "search")]
                lock.search_mode,
                lock.line_numbers,
                lock.rows,
            );
            #[allow(clippy::match_same_arms)]
            match input {
                Some(InputEvent::Exit) => {
                    lock.exit();
                    return Ok(cleanup(out, &lock.exit_strategy, true)?);
                }
                Some(InputEvent::UpdateTermArea(c, r)) => {
                    lock.cols = c;
                    lock.rows = r;
                    lock.readjust_wraps();
                    redraw = true;
                }
                Some(InputEvent::UpdateUpperMark(um)) => {
                    lock.upper_mark = um;
                    redraw = true;
                }
                Some(InputEvent::UpdateLineNumber(l)) => {
                    lock.line_numbers = l;
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(InputEvent::Search(m)) => {
                    lock.search_mode = m;
                    // Fetch the search query asynchronously
                    let string = search::fetch_input(&mut out, lock.search_mode, lock.rows).await?;
                    // If the string is not empty, highlight all instances of the
                    // match and return a vector of match coordinates
                    if !string.is_empty() {
                        lock.search_term = Some(regex::Regex::new(&string)?);
                        search::highlight_search(&mut lock);
                    }
                    search::next_match(&mut lock, &mut s_mark);
                    // Update the search term
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(InputEvent::NextMatch) if lock.search_term.is_some() => {
                    // Increment the search mark only if it is less than s_co.len
                    // and it is not the last page
                    if s_mark < lock.search_idx.len().saturating_sub(1)
                        && lock.upper_mark + lock.rows < lock.num_lines()
                    {
                        s_mark += 1;
                    }
                    if lock.search_idx.len() > s_mark {
                        search::next_match(&mut lock, &mut s_mark);
                        redraw = true;
                    }
                }
                #[cfg(feature = "search")]
                Some(InputEvent::PrevMatch) if lock.search_term.is_some() => {
                    if lock.search_idx.is_empty() {
                        continue;
                    }
                    s_mark = s_mark.saturating_sub(1);
                    // Get the search line
                    let y = lock.search_idx[s_mark];
                    // If it's passed by the user, go back to it
                    if usize::from(y) <= lock.upper_mark {
                        lock.upper_mark = y.into();
                    }
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(_) => continue,
                None => continue,
            }
            // If redraw is true, then redraw the screen
            if redraw {
                draw(&mut out, &mut lock)?;
            }
        }
    }
}
