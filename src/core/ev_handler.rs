//! Provides the [`handle_event`] function

use std::convert::TryInto;
use std::io::Write;
use std::sync::{atomic::AtomicBool, Arc};

#[cfg(feature = "search")]
use parking_lot::{Condvar, Mutex};

use super::utils::display;
use super::{commands::Command, utils::term};
#[cfg(feature = "search")]
use crate::search;
use crate::{error::MinusError, input::InputEvent, PagerState};

/// Respond based on the type of event
///
/// It will match the type of event received and based on that, it can take actions like:-
/// - Mutating fields of [`PagerState`]
/// - Handle cleanup and exits
/// - Call search related functions
#[cfg_attr(not(feature = "search"), allow(unused_mut))]
#[allow(clippy::too_many_lines)]
pub fn handle_event(
    ev: Command,
    mut out: &mut impl Write,
    p: &mut PagerState,
    is_exited: &Arc<AtomicBool>,
    #[cfg(feature = "search")] user_input_active: &Arc<(Mutex<bool>, Condvar)>,
) -> Result<(), MinusError> {
    match ev {
        Command::SetData(text) => {
            p.screen.orig_text = text;
            p.format_lines();
        }
        Command::UserInput(InputEvent::Exit) => {
            p.exit();
            is_exited.store(true, std::sync::atomic::Ordering::SeqCst);
            term::cleanup(&mut out, &p.exit_strategy, true)?;
        }
        Command::UserInput(InputEvent::UpdateUpperMark(mut um)) => {
            display::draw_for_change(out, p, &mut um)?;
            p.upper_mark = um;
        }
        Command::UserInput(InputEvent::RestorePrompt) => {
            // Set the message to None and new messages to false as all messages have been shown
            p.message = None;
            p.format_prompt();
        }
        Command::UserInput(InputEvent::UpdateTermArea(c, r)) => {
            p.rows = r;
            p.cols = c;
            // Readjust the text wrapping for the new number of columns
            p.format_lines();
        }
        Command::UserInput(InputEvent::UpdateLineNumber(l)) => {
            p.line_numbers = l;
            p.format_lines();
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::Search(m)) => {
            p.search_mode = m;
            p.search_state.search_mode = m;
            // Reset search mark so it won't be out of bounds if we have
            // less matches in this search than last time
            p.search_state.search_mark = 0;

            // Pause the main user input thread, read search query and then restart the main input thread
            let (lock, cvar) = (&user_input_active.0, &user_input_active.1);
            let mut active = lock.lock();
            *active = false;
            drop(active);
            // let string = search::fetch_input(&mut out, p.search_mode, p.rows)?;
            let search_result = search::fetch_input(&mut out, p)?;
            let mut active = lock.lock();
            *active = true;
            drop(active);
            cvar.notify_one();

            // If we have incremental search cache directly use it and return
            if let Some(incremental_search_result) = search_result.incremental_search_result {
                p.search_state.search_term = search_result.compiled_regex;
                p.upper_mark = incremental_search_result.upper_mark;
                p.search_state.search_mark = incremental_search_result.search_mark;
                p.search_state.search_idx = incremental_search_result.search_idx;
                p.screen.formatted_lines = incremental_search_result.formatted_lines;
                return Ok(());
            }

            // If we only have compiled regex cached, use that otherwise compile the original
            // string query if its not empty
            p.search_state.search_term = if search_result.compiled_regex.is_some() {
                search_result.compiled_regex
            } else if !search_result.string.is_empty() {
                let compiled_regex = regex::Regex::new(&search_result.string).ok();
                if compiled_regex.is_none() {
                    p.message = Some("Invalid regular expression. Press Enter".to_owned());
                    p.format_prompt();
                }
                compiled_regex
            } else {
                return Ok(());
            };

            // Format the lines, this will automatically generate the PagerState.search_idx
            p.format_lines();

            // Move to next search match after the current upper_mark
            let position_of_next_match =
                search::next_nth_match(&p.search_state.search_idx, p.upper_mark, 0);

            if let Some(pnm) = position_of_next_match {
                p.search_state.search_mark = pnm;
                p.upper_mark = *p
                    .search_state
                    .search_idx
                    .iter()
                    .nth(p.search_state.search_mark)
                    .unwrap();
            }

            p.format_prompt();
            display::draw_full(&mut out, p)?;
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::NextMatch | InputEvent::MoveToNextMatch(1))
            if p.search_state.search_term.is_some() =>
        {
            // Move to next search match after the current upper_mark
            let position_of_next_match =
                search::next_nth_match(&p.search_state.search_idx, p.upper_mark, 1);
            if let Some(pnm) = position_of_next_match {
                p.search_state.search_mark = pnm;
                p.upper_mark = *p
                    .search_state
                    .search_idx
                    .iter()
                    .nth(p.search_state.search_mark)
                    .unwrap();
            }

            p.format_prompt();
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::PrevMatch | InputEvent::MoveToPrevMatch(1))
            if p.search_state.search_term.is_some() =>
        {
            // If no matches, return immediately
            if p.search_state.search_idx.is_empty() {
                return Ok(());
            }
            // Decrement the s_mark and get the preceding index
            p.search_state.search_mark = p.search_state.search_mark.saturating_sub(1);
            if let Some(y) = p
                .search_state
                .search_idx
                .iter()
                .nth(p.search_state.search_mark)
            {
                // If the index is less than or equal to the upper_mark, then set y to the new upper_mark
                if *y < p.upper_mark {
                    p.upper_mark = *y;
                    p.format_prompt();
                }
            }
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::MoveToNextMatch(n))
            if p.search_state.search_term.is_some() =>
        {
            // Move to next nth search match after the current upper_mark
            let position_of_next_match =
                search::next_nth_match(&p.search_state.search_idx, p.upper_mark, n);
            if let Some(pnm) = position_of_next_match {
                p.search_state.search_mark = pnm;
                p.upper_mark = *p
                    .search_state
                    .search_idx
                    .iter()
                    .nth(p.search_state.search_mark)
                    .unwrap();

                // Ensure there is enough text available after location corresponding to
                // position_of_next_match so that we can display a pagefull of data. If not,
                // reduce it so that a pagefull of text can be accommodated.
                // NOTE: Add 1 to total number of lines to avoid off-by-one errors
                while p.upper_mark.saturating_add(p.rows)
                    > p.screen.formatted_lines_count().saturating_add(1)
                {
                    p.search_state.search_mark = p.search_state.search_mark.saturating_sub(1);
                    p.upper_mark = *p
                        .search_state
                        .search_idx
                        .iter()
                        .nth(p.search_state.search_mark)
                        .unwrap();
                }
            }
            p.format_prompt();
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::MoveToPrevMatch(n))
            if p.search_state.search_term.is_some() =>
        {
            // If no matches, return immediately
            if p.search_state.search_idx.is_empty() {
                return Ok(());
            }
            // Decrement the s_mark and get the preceding index
            p.search_state.search_mark = p.search_state.search_mark.saturating_sub(n);
            if let Some(y) = p
                .search_state
                .search_idx
                .iter()
                .nth(p.search_state.search_mark)
            {
                // If the index is less than or equal to the upper_mark, then set y to the new upper_mark
                if *y < p.upper_mark {
                    p.upper_mark = *y;
                    p.format_prompt();
                }
            }
        }

        Command::AppendData(text) => {
            let prev_unterminated = p.unterminated;
            let prev_fmt_lines_count = p.screen.formatted_lines_count();
            let append_style = p.append_str(text.as_str());
            if !p.running.lock().is_uninitialized() {
                display::draw_append_text(
                    out,
                    p,
                    prev_unterminated,
                    prev_fmt_lines_count,
                    append_style,
                )?;
                return Ok(());
            }
        }

        Command::SetPrompt(ref text) | Command::SendMessage(ref text) => {
            if let Command::SetPrompt(_) = ev {
                p.prompt = text.to_string();
            } else {
                p.message = Some(text.to_string());
            }
            p.format_prompt();
            term::move_cursor(&mut out, 0, p.rows.try_into().unwrap(), false)?;
            if !p.running.lock().is_uninitialized() {
                super::utils::display::write_prompt(
                    &mut out,
                    &p.displayed_prompt,
                    p.rows.try_into().unwrap(),
                )?;
            }
        }
        Command::SetLineNumbers(ln) => {
            p.line_numbers = ln;
            p.format_lines();
        }
        Command::SetExitStrategy(es) => p.exit_strategy = es,
        #[cfg(feature = "static_output")]
        Command::SetRunNoOverflow(val) => p.run_no_overflow = val,
        #[cfg(feature = "search")]
        Command::IncrementalSearchCondition(cb) => p.search_state.incremental_search_condition = cb,
        Command::SetInputClassifier(clf) => p.input_classifier = clf,
        Command::AddExitCallback(cb) => p.exit_callbacks.push(cb),
        Command::ShowPrompt(show) => p.show_prompt = show,
        Command::UserInput(_) => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::commands::Command;
    use super::handle_event;
    use crate::{ExitStrategy, PagerState};
    use std::sync::{atomic::AtomicBool, Arc};
    #[cfg(feature = "search")]
    use {
        once_cell::sync::Lazy,
        parking_lot::{Condvar, Mutex},
    };

    // Tests constants
    #[cfg(feature = "search")]
    static UIA: Lazy<Arc<(Mutex<bool>, Condvar)>> =
        Lazy::new(|| Arc::new((Mutex::new(true), Condvar::new())));
    const TEST_STR: &str = "This is some sample text";

    // Tests for event emitting functions of Pager
    #[test]
    fn set_data() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetData(TEST_STR.to_string());
        let mut out = Vec::new();

        handle_event(
            ev,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        assert_eq!(ps.screen.formatted_lines, vec![TEST_STR.to_string()]);
    }

    #[test]
    fn append_str() {
        let mut ps = PagerState::new().unwrap();
        let ev1 = Command::AppendData(format!("{TEST_STR}\n"));
        let ev2 = Command::AppendData(TEST_STR.to_string());
        let mut out = Vec::new();

        handle_event(
            ev1,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        handle_event(
            ev2,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        assert_eq!(
            ps.screen.formatted_lines,
            vec![TEST_STR.to_string(), TEST_STR.to_string()]
        );
    }

    #[test]
    fn set_prompt() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetPrompt(TEST_STR.to_string());
        let mut out = Vec::new();

        handle_event(
            ev,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        assert_eq!(ps.prompt, TEST_STR.to_string());
    }

    #[test]
    fn send_message() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SendMessage(TEST_STR.to_string());
        let mut out = Vec::new();

        handle_event(
            ev,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        assert_eq!(ps.message.unwrap(), TEST_STR.to_string());
    }

    #[test]
    #[cfg(feature = "static_output")]
    fn set_run_no_overflow() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetRunNoOverflow(false);
        let mut out = Vec::new();

        handle_event(
            ev,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        assert!(!ps.run_no_overflow);
    }

    #[test]
    fn set_exit_strategy() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetExitStrategy(ExitStrategy::PagerQuit);
        let mut out = Vec::new();

        handle_event(
            ev,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        assert_eq!(ps.exit_strategy, ExitStrategy::PagerQuit);
    }

    #[test]
    fn add_exit_callback() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::AddExitCallback(Box::new(|| println!("Hello World")));
        let mut out = Vec::new();

        handle_event(
            ev,
            &mut out,
            &mut ps,
            &Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "search")]
            &UIA,
        )
        .unwrap();
        assert_eq!(ps.exit_callbacks.len(), 1);
    }
}
