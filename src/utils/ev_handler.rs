use std::io::Stdout;

use super::term::cleanup;
#[cfg(feature = "search")]
use crate::search;
use crate::{error::AlternateScreenPagingError, input::InputEvent, Pager};

// This file contains the handle_input function to handle events

// This function matches the given Option<InputEvent> and handles the event appropriately
pub(crate) fn handle_input(
    ev: &Option<InputEvent>,
    mut pager: &mut Pager,
    mut out: &mut Stdout,
    redraw: &mut bool,
    #[cfg(feature = "search")] mut s_mark: &mut usize,
) -> Result<(), AlternateScreenPagingError> {
    #[allow(clippy::match_same_arms)]
    match ev {
        Some(InputEvent::Exit) => {
            pager.exit();
            return Ok(cleanup(out, &pager.exit_strategy, true)?);
        }
        Some(InputEvent::RestorePrompt) => {
            // Set the message to None and new messages to false as all messages have been shown
            pager.message.0 = None;
            pager.message.1 = false;
            *redraw = true;
        }
        Some(InputEvent::UpdateTermArea(c, r)) => {
            pager.rows = *r;
            pager.cols = *c;
            // Readjust the text wrapping for the new number of columns
            pager.readjust_wraps();
            *redraw = true;
        }
        Some(InputEvent::UpdateUpperMark(um)) => {
            pager.upper_mark = *um;
            *redraw = true;
        }
        Some(InputEvent::UpdateLineNumber(l)) => {
            pager.line_numbers = *l;
            *redraw = true;
        }
        #[cfg(feature = "search")]
        Some(InputEvent::Search(m)) => {
            pager.search_mode = *m;
            // Get the query
            let string = search::fetch_input(&mut out, pager.search_mode, pager.rows)?;
            if !string.is_empty() {
                let regex = regex::Regex::new(&string);
                if let Ok(r) = regex {
                    pager.search_term = Some(r);
                    // Prepare a index where search matches are found
                    // and set it to pager.search_idx
                    search::set_match_indices(&mut pager);
                    // Move to
                    search::next_match(&mut pager, &mut s_mark);
                } else {
                    // Send invalid regex message at the prompt if invalid regex is given
                    pager.send_message("Invalid regular expression. Press Enter");
                }
            }
            *redraw = true;
        }
        #[cfg(feature = "search")]
        Some(InputEvent::NextMatch) if pager.search_term.is_some() => {
            // If s_mark is less than the length of pager.search_idx
            // and it is not page where the last match is present
            // then increment the s_mark
            if *s_mark < pager.search_idx.len().saturating_sub(1)
                && pager.upper_mark + pager.rows < pager.num_lines()
            {
                *s_mark += 1;
            }
            // Go to the next match
            search::next_match(&mut pager, &mut s_mark);
            *redraw = true;
        }
        #[cfg(feature = "search")]
        Some(InputEvent::PrevMatch) if pager.search_term.is_some() => {
            // If no matches, return immediately
            if pager.search_idx.is_empty() {
                return Ok(());
            }
            // Decrement the s_mark and get the preceeding index
            *s_mark = s_mark.saturating_sub(1);
            let y = pager.search_idx[*s_mark];
            // If the index is less than or equal to the upper_mark, then set y to the new upper_mark
            if y < pager.upper_mark {
                pager.upper_mark = y;
            }
            *redraw = true;
        }
        #[cfg(feature = "search")]
        Some(_) => return Ok(()),
        None => return Ok(()),
    }
    Ok(())
}
