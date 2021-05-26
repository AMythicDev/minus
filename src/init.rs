// There maybe a lot of junk that may get imported if not needed. So we allow
// unused imports
#![allow(unused_imports)]

#[cfg(feature = "search")]
use crate::search;
#[cfg(feature = "search")]
use crate::utils::SearchMode;
use crate::utils::{cleanup, draw, handle_input, setup, InputEvent};
use crate::{error::AlternateScreenPagingError, Pager};
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use async_mutex::Mutex;
use crossterm::{cursor::MoveTo, event, execute};
#[cfg(feature = "search")]
use std::convert::{TryFrom, TryInto};
use std::io::{self, Write as _};
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use std::sync::Arc;

#[cfg(feature = "static_output")]
#[allow(clippy::clippy::too_many_lines)]
pub(crate) fn static_paging(mut pager: Pager) -> Result<(), AlternateScreenPagingError> {
    let mut out = io::stdout();
    setup(&out, false)?;
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
            let input = handle_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                &pager,
            );
            // Update any data that may have changed
            #[allow(clippy::clippy::match_same_arms)]
            match input {
                Some(InputEvent::Exit) => return Ok(cleanup(out, &pager.exit_strategy)?),
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
                        pager.search_term = string;
                        search::highlight_search(&mut pager)
                            .map_err(|e| AlternateScreenPagingError::SearchExpError(e.into()))?;
                    }
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(InputEvent::NextMatch) if !pager.search_term.is_empty() => {
                    if s_mark < pager.search_idx.len().saturating_sub(1)
                        && pager.upper_mark + pager.rows < pager.num_lines()
                    {
                        s_mark += 1;
                    }
                    if pager.search_idx.len() > s_mark {
                        while let Some(y) = pager.search_idx.get(s_mark) {
                            if usize::from(*y) < pager.upper_mark {
                                s_mark += 1;
                            } else {
                                pager.upper_mark = (*y).into();
                                break;
                            }
                        }
                        redraw = true;
                    }
                }
                #[cfg(feature = "search")]
                Some(InputEvent::PrevMatch) if !pager.search_term.is_empty() => {
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
/// `get` is a function that will extract the Pager lock from the
/// `PageMutex`. `get` is only called when drawing, Therefore, it can be mutated the entire time, except while drawing
///
/// ## Errors
///
/// Setting/cleaning up the terminal can fail and IO to/from the terminal can
/// fail.
#[cfg(any(feature = "async_std_lib", feature = "tokio_lib"))]
#[allow(clippy::clippy::too_many_lines)]
pub(crate) async fn dynamic_paging(
    p: &Arc<Mutex<Pager>>,
) -> std::result::Result<(), AlternateScreenPagingError> {
    // Setup terminal, adjust line wraps and get rows
    let mut out = io::stdout();
    setup(&out, true)?;
    let mut guard = p.lock().await;
    guard.prepare()?;
    let mut rows = guard.rows;
    drop(guard);
    // Search related variables
    // Vector of match coordinates

    // A marker of which element of s_co we are currently at
    // -1 means we have just highlighted all the matches but the cursor has not been
    // placed in any one of them
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
        let line_count = guard.lines.len();
        let have_just_overflowed = (last_line_count < rows) && (line_count >= rows);
        if last_line_count != line_count && (line_count < rows || have_just_overflowed) {
            draw(&mut out, &mut guard)?;
            last_line_count = line_count;
        }

        drop(guard);
        // Check for events
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            // Lock the value again
            let mut lock = p.lock().await;

            // Get the events
            let input = handle_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                &lock,
            );
            // Update any data that may have changed
            #[allow(clippy::clippy::match_same_arms)]
            match input {
                Some(InputEvent::Exit) => return Ok(cleanup(&mut out, &lock.exit_strategy)?),
                Some(InputEvent::UpdateTermArea(c, r)) => {
                    rows = r;
                    lock.rows = r;
                    lock.cols = c;
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
                        search::highlight_search(&mut lock)
                            .map_err(|e| AlternateScreenPagingError::SearchExpError(e.into()))?;
                        lock.search_term = string;
                    }
                    // Update the search term
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(InputEvent::NextMatch) if !lock.search_term.is_empty() => {
                    // Increment the search mark only if it is less than s_co.len
                    // and it is not the last page
                    if s_mark < lock.search_idx.len().saturating_sub(1)
                        && lock.upper_mark + lock.rows < lock.num_lines()
                    {
                        s_mark += 1;
                    }
                    if lock.search_idx.len() > s_mark {
                        // Get the search line
                        while let Some(y) = lock.search_idx.get(s_mark) {
                            // If the line is already paged down by the user
                            // move for the next line
                            if usize::from(*y) < lock.upper_mark {
                                s_mark += 1;
                            } else {
                                // If the line is at the lower position than the top line
                                // make it the next top line
                                lock.upper_mark = (*y).into();
                                break;
                            }
                        }
                        redraw = true;
                    }
                }
                #[cfg(feature = "search")]
                Some(InputEvent::PrevMatch) if !lock.search_term.is_empty() => {
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
