#[cfg(feature = "search")]
use crate::search;
#[cfg(feature = "search")]
use crate::utils::SearchMode;
use crate::utils::{cleanup, draw, handle_input, setup, InputEvent};
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use crate::PagerMutex;
use crate::{error::AlternateScreenPagingError, Pager};
use crossterm::{cursor::MoveTo, event};
#[cfg(feature = "search")]
use std::convert::{TryFrom, TryInto};
use std::io::{self, Write as _};
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
use std::sync::Arc;

#[cfg(feature = "static_output")]
#[allow(clippy::clippy::too_many_lines)]
pub(crate) fn static_paging(mut pager: Pager) -> Result<(), AlternateScreenPagingError> {
    // Setup terminal
    let mut out = io::stdout();
    let mut rows = setup(&out, false)?;
    #[allow(unused_assignments)]
    let mut redraw = true;

    #[cfg(feature = "search")]
    let mut s_co: Vec<(u16, u16)> = Vec::new();
    #[cfg(feature = "search")]
    let mut s_mark = -1;
    #[cfg(feature = "search")]
    let mut search_mode = SearchMode::Unknown;

    draw(&mut out, &mut pager, rows)?;

    loop {
        // Check for events
        if event::poll(std::time::Duration::from_millis(10))
            .map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?
        {
            // Get the events
            let input = handle_input(
                event::read().map_err(|e| AlternateScreenPagingError::HandleEvent(e.into()))?,
                pager.upper_mark,
                search_mode,
                pager.line_numbers,
                rows,
            );
            // Update any data that may have changed
            match input {
                Some(InputEvent::Exit) => return Ok(cleanup(out, &pager.exit_strategy)?),
                Some(InputEvent::UpdateRows(r)) => {
                    rows = r;
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
                #[cfg(feature = "search")]
                Some(InputEvent::Search(m)) => {
                    search_mode = m;
                    let string = search::fetch_input_blocking(&mut out, search_mode, rows)?;
                    if !string.is_empty() {
                        s_co = search::highlight_search(&mut pager, &string)
                            .map_err(|e| AlternateScreenPagingError::SearchExpError(e.into()))?;
                        pager.search_term = string;
                    }
                    redraw = true;
                }
                // These are same as their dynamic counterparts, except for the fact
                // that they use a blocking fetch_input function
                #[cfg(feature = "search")]
                Some(InputEvent::NextMatch) if !pager.search_term.is_empty() => {
                    // Increment the search mark
                    s_mark += 1;
                    // These unwrap operations should be safe, error handling for these
                    // could be added later on
                    // Make sure s_mark is not greater than s_co's lenght
                    if isize::try_from(s_co.len()).unwrap() > s_mark {
                        // Get the next coordinates
                        let (mut x, mut y) = s_co[usize::try_from(s_mark).unwrap()];
                        if usize::from(y) < pager.upper_mark {
                            s_mark += 1;
                            while let Some(co) = s_co.get(usize::try_from(s_mark).unwrap()) {
                                if usize::from(co.1) <= pager.upper_mark {
                                    s_mark += 1;
                                } else {
                                    x = co.0;
                                    y = co.1;
                                    break;
                                }
                            }
                        }
                        if usize::from(y) >= pager.upper_mark + rows {
                            pager.upper_mark = y.into();
                        }
                        draw(&mut out, &mut pager, rows)?;
                        y = y.saturating_sub(pager.upper_mark.try_into().unwrap());
                        write!(out, "{}", MoveTo(x, y))?;
                        out.flush()?;
                        // Do not redraw the console
                        redraw = false;
                    }
                }
                #[cfg(feature = "search")]
                Some(InputEvent::PrevMatch) if !pager.search_term.is_empty() => {
                    if isize::try_from(s_co.len()).unwrap() > s_mark {
                        // If s_mark is less than 0, make it 0, else subtract 1 from it
                        s_mark = if s_mark <= 0 {
                            0
                        } else {
                            s_mark.saturating_sub(1)
                        };
                        // Do the same steps that we have did in NextMatch block
                        let (x, mut y) = s_co[usize::try_from(s_mark).unwrap()];
                        if usize::from(y) <= pager.upper_mark {
                            pager.upper_mark = y.into();
                        }
                        draw(&mut out, &mut pager, rows)?;
                        y = y.saturating_sub(pager.upper_mark.try_into().unwrap());

                        write!(out, "{}", MoveTo(x, y))?;
                        out.flush()?;
                        redraw = false;
                    }
                }
                #[cfg(feature = "search")]
                None | Some(_) => continue,
            }
            if redraw {
                draw(&mut out, &mut pager, rows)?;
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
    p: &Arc<PagerMutex>,
) -> std::result::Result<(), AlternateScreenPagingError> {
    // Setup terminal
    let mut out = io::stdout();
    let mut rows = setup(&out, true)?;
    // Search related variables
    // Vector of match coordinates
    // Earch element is a (x,y) pair, where the cursor will be placed
    #[cfg(feature = "search")]
    let mut s_co: Vec<(u16, u16)> = Vec::new();
    // A marker of which element of s_co we are currently at
    // -1 means we have just highlighted all the matches but the cursor has not been
    // placed in any one of them
    #[cfg(feature = "search")]
    let mut s_mark = -1;
    // Search Mode
    #[cfg(feature = "search")]
    let mut search_mode = SearchMode::Unknown;
    // Whether to redraw the console
    #[allow(unused_assignments)]
    let mut redraw = true;
    let mut last_line_count = 0;

    loop {
        // Get the lock, clone it and immidiately drop the lock
        let mut guard = p.lock().await;

        // Display the text continously if last displayed line count is not same and
        // all rows are not filled
        let line_count = guard.lines.lines().count();
        if last_line_count != line_count && line_count < rows {
            draw(&mut out, &mut guard, rows)?;
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
                lock.upper_mark,
                search_mode,
                lock.line_numbers,
                rows,
            );
            // Update any data that may have changed
            match input {
                Some(InputEvent::Exit) => return Ok(cleanup(out, &lock.exit_strategy)?),
                Some(InputEvent::UpdateRows(r)) => {
                    rows = r;
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
                Some(InputEvent::Search(m)) if lock.searchable => {
                    search_mode = m;
                    // Fetch the search query asynchronously
                    let string = search::fetch_input(&mut out, search_mode, rows).await?;
                    if !string.is_empty() {
                        // If the string is not empty, highlight all instances of the
                        // match and return a vector of match coordinates
                        s_co = search::highlight_search(&mut lock, &string)
                            .map_err(|e| AlternateScreenPagingError::SearchExpError(e.into()))?;
                        // Update the search term
                        lock.search_term = string;
                    }
                    redraw = true;
                }
                #[cfg(feature = "search")]
                Some(InputEvent::NextMatch) if !lock.search_term.is_empty() => {
                    // Increment the search mark
                    s_mark += 1;
                    // These unwrap operations should be safe
                    // Make sure s_mark is not greater than s_co's lenght
                    if isize::try_from(s_co.len()).unwrap() > s_mark {
                        // Get the next coordinates
                        let (mut x, mut y) = s_co[usize::try_from(s_mark).unwrap()];
                        if usize::from(y) < lock.upper_mark {
                            s_mark += 1;
                            // Make sure that the next match taken to is after the
                            // current upper_mark
                            while let Some(co) = s_co.get(usize::try_from(s_mark).unwrap()) {
                                if usize::from(co.1) <= lock.upper_mark {
                                    s_mark += 1;
                                } else {
                                    x = co.0;
                                    y = co.1;
                                    break;
                                }
                            }
                        }
                        if usize::from(y) >= lock.upper_mark + rows {
                            lock.upper_mark = y.into();
                        }
                        draw(&mut out, &mut lock, rows)?;
                        y = y.saturating_sub(lock.upper_mark.try_into().unwrap());

                        write!(out, "{}", MoveTo(x, y))?;
                        out.flush()?;
                        // Do not redraw the console
                        redraw = false;
                    }
                }
                #[cfg(feature = "search")]
                Some(InputEvent::PrevMatch) if !lock.search_term.is_empty() => {
                    if isize::try_from(s_co.len()).unwrap() > s_mark {
                        // If s_mark is less than 0, make it 0, else subtract 1 from it
                        s_mark = if s_mark <= 0 {
                            0
                        } else {
                            s_mark.saturating_sub(1)
                        };
                        // Do the same steps that we have did in NextMatch block
                        let (x, mut y) = s_co[usize::try_from(s_mark).unwrap()];
                        if usize::from(y) <= lock.upper_mark {
                            lock.upper_mark = y.into();
                        }
                        draw(&mut out, &mut lock, rows)?;
                        y = y.saturating_sub(lock.upper_mark.try_into().unwrap());

                        write!(out, "{}", MoveTo(x, y))?;
                        out.flush()?;
                        redraw = false;
                    }
                }
                None | Some(_) => continue,
            }
            // If redraw is true, then redraw the screen
            if redraw {
                draw(&mut out, &mut lock, rows)?;
            }
        }
    }
}
