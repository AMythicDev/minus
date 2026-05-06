//! Provides the [`handle_event`] function

use std::convert::TryInto;
use std::io::Write;
use std::sync::{Arc, atomic::AtomicBool};

#[cfg(feature = "search")]
use parking_lot::{Condvar, Mutex};

use super::CommandQueue;
use super::commands::{Command, InternalCommand};
use super::utils::display::{self, AppendStyle};
#[cfg(feature = "search")]
use crate::search;
use crate::{PagerState, error::MinusError, input::InputEvent};

/// Respond based on the type of command
///
/// It will match the type of event received and based on that, it can take actions like:-
/// - Mutating fields of [`PagerState`]
/// - Handle cleanup and exits
/// - Call search related functions
#[cfg_attr(not(feature = "search"), allow(unused_mut))]
#[allow(clippy::too_many_lines)]
pub fn handle_event(
    ev: Command,
    p: &mut PagerState,
    command_queue: &mut CommandQueue,
    is_exited: &Arc<AtomicBool>,
) {
    match ev {
        Command::SetData(text) => {
            p.screen.orig_text = text;
            p.screen.line_count = p.screen.orig_text.lines().count();
            p.format_lines();
            command_queue.push_back(Command::Internal(InternalCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::Exit) => {
            p.exit();
            is_exited.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Command::UserInput(InputEvent::UpdateUpperMark(um)) => {
            command_queue.push_back(Command::Internal(InternalCommand::SetUpperMark(um)));
        }
        Command::UserInput(InputEvent::UpdateLeftMark(lm)) if !p.screen.line_wrapping => {
            if lm.saturating_add(p.cols) > p.screen.get_max_line_length() && lm > p.left_mark {
                return;
            }
            p.left_mark = lm;
            command_queue.push_back(Command::Internal(InternalCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::RestorePrompt) => {
            // Set the message to None and new messages to false as all messages have been shown
            p.message = None;
            p.format_prompt();
            command_queue.push_back_unchecked(Command::Internal(InternalCommand::RedrawPrompt));
        }
        Command::UserInput(InputEvent::UpdateTermArea(c, r)) => {
            p.rows = r;
            p.cols = c;
            p.format_lines();
            // Readjust the text wrapping for the new number of columns
            command_queue.push_back(Command::Internal(InternalCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::UpdateLineNumber(l)) => {
            p.line_numbers = l;
            p.format_lines();
            command_queue.push_back(Command::Internal(InternalCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::Number(n)) => {
            p.prefix_num.push(n);
            p.format_prompt();
            command_queue.push_back(Command::Internal(InternalCommand::RedrawPrompt));
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::Search(m)) => {
            p.search_mode = m;
            p.search_state.search_mode = m;
            p.search_state.search_mark = 0;
            command_queue.push_back(Command::Internal(InternalCommand::FetchSearchQuery));
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
                let upper_mark = *p
                    .search_state
                    .search_idx
                    .iter()
                    .nth(p.search_state.search_mark)
                    .unwrap();
                command_queue.push_back_unchecked(Command::Internal(
                    InternalCommand::SetUpperMark(upper_mark),
                ));
                p.format_prompt();
                command_queue.push_back(Command::Internal(InternalCommand::RedrawPrompt));
            }
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::PrevMatch | InputEvent::MoveToPrevMatch(1))
            if p.search_state.search_term.is_some() =>
        {
            // If no matches, return immediately
            if p.search_state.search_idx.is_empty() {
                return;
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
                    command_queue.push_back(Command::UserInput(InputEvent::UpdateUpperMark(*y)));
                    p.format_prompt();
                    command_queue.push_back(Command::Internal(InternalCommand::RedrawPrompt));
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
                let mut upper_mark = *p
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
                    upper_mark = *p
                        .search_state
                        .search_idx
                        .iter()
                        .nth(p.search_state.search_mark)
                        .unwrap();
                }
                command_queue
                    .push_back(Command::UserInput(InputEvent::UpdateUpperMark(upper_mark)));
                p.format_prompt();
                command_queue.push_back(Command::Internal(InternalCommand::RedrawPrompt));
            }
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::MoveToPrevMatch(n))
            if p.search_state.search_term.is_some() =>
        {
            // If no matches, return immediately
            if p.search_state.search_idx.is_empty() {
                return;
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
                    command_queue.push_back(Command::Internal(InternalCommand::SetUpperMark(*y)));
                    p.format_prompt();
                    command_queue.push_back(Command::Internal(InternalCommand::RedrawPrompt));
                }
            }
        }

        Command::UserInput(InputEvent::HorizontalScroll(val)) => {
            p.screen.line_wrapping = val;
            p.format_lines();
            command_queue.push_back_unchecked(Command::Internal(InternalCommand::RedrawDisplay));
        }

        Command::AppendData(text) => {
            let prev_unterminated = p.screen.unterminated;
            let prev_fmt_lines_count = p.screen.formatted_lines_count();
            let append_style = p.append_str(text.as_str());

            if append_style == AppendStyle::FullRedraw {
                command_queue.push_back(Command::Internal(InternalCommand::RedrawDisplay));
                return;
            }

            command_queue.push_back(Command::Internal(InternalCommand::DrawAppendedText(
                prev_unterminated,
                prev_fmt_lines_count,
                append_style,
            )));

            if p.follow_output {
                command_queue.push_back_unchecked(Command::Internal(
                    InternalCommand::SetUpperMark(p.screen.formatted_lines_count()),
                ));
            }
        }

        Command::SetPrompt(ref text) | Command::SendMessage(ref text) => {
            if let Command::SetPrompt(_) = ev {
                p.prompt = text.clone();
            } else {
                p.message = Some(text.clone());
            }
            p.format_prompt();
            command_queue.push_back(Command::Internal(InternalCommand::RedrawPrompt));
        }
        Command::SetLineNumbers(ln) => {
            p.line_numbers = ln;
            p.format_lines();
            command_queue.push_back(Command::Internal(InternalCommand::RedrawDisplay));
        }
        Command::SetExitStrategy(es) => p.exit_strategy = es,
        Command::LineWrapping(lw) => {
            p.screen.line_wrapping = lw;
            p.format_lines();
        }
        #[cfg(feature = "static_output")]
        Command::SetRunNoOverflow(val) => p.run_no_overflow = val,
        #[cfg(feature = "search")]
        Command::IncrementalSearchCondition(cb) => p.search_state.incremental_search_condition = cb,
        Command::SetInputClassifier(clf) => p.input_classifier = clf,
        Command::AddExitCallback(cb) => p.exit_callbacks.push(cb),
        Command::ShowPrompt(show) => p.show_prompt = show,
        Command::FollowOutput(follow_output)
        | Command::UserInput(InputEvent::FollowOutput(follow_output)) => {
            p.follow_output = follow_output;
            command_queue.push_back(Command::UserInput(InputEvent::UpdateUpperMark(
                p.screen.formatted_lines_count(),
            )));
            p.format_prompt();
            command_queue.push_back(Command::Internal(InternalCommand::RedrawPrompt));
        }
        Command::UserInput(_) => {}
        Command::Internal(_) => unreachable!(),
    }
}

#[cfg_attr(
    not(feature = "search"),
    allow(unused_variables),
    allow(clippy::needless_pass_by_ref_mut)
)]
pub fn handle_internal_command(
    internal_command: InternalCommand,
    mut out: &mut impl Write,
    p: &mut PagerState,
    command_queue: &mut CommandQueue,
    #[cfg(feature = "search")] user_input_active: &Arc<(Mutex<bool>, Condvar)>,
) -> Result<(), MinusError> {
    if p.running.lock().is_uninitialized() {
        return Ok(());
    }
    match internal_command {
        InternalCommand::RedrawPrompt => {
            display::write_prompt(out, &p.displayed_prompt, p.rows.try_into().unwrap())?;
        }
        InternalCommand::RedrawDisplay => {
            display::draw_full(&mut out, p)?;
        }
        InternalCommand::SetUpperMark(mut um) => {
            display::draw_for_change(out, p, &mut um)?;
            p.upper_mark = um;
        }
        InternalCommand::DrawAppendedText(
            prev_unterminated,
            prev_fmt_lines_count,
            append_style,
        ) => {
            let AppendStyle::PartialUpdate(bounds) = append_style else {
                unreachable!();
            };
            let fmt_lines = p.screen.get_formatted_lines_with_bounds(bounds.0, bounds.1);
            display::draw_append_text(
                out,
                p.rows,
                prev_unterminated,
                prev_fmt_lines_count,
                fmt_lines,
            )?;
        }
        #[cfg(feature = "search")]
        InternalCommand::FetchSearchQuery => {
            // Pause the main user input thread, read search query and then restart the main input thread
            let (lock, cvar) = (&user_input_active.0, &user_input_active.1);
            let mut active = lock.lock();
            *active = false;
            drop(active);
            cvar.notify_one();
            let search_result = search::fetch_input(&mut out, p)?;
            let mut active = lock.lock();
            *active = true;
            drop(active);
            cvar.notify_one();

            command_queue.push_back_unchecked(Command::Internal(InternalCommand::RedrawPrompt));
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
                    command_queue.push_back_unchecked(Command::SendMessage(
                        "Invalid regular expression. Press Enter".to_string(),
                    ));
                    return Ok(());
                }
                compiled_regex
            } else {
                return Ok(());
            };

            p.format_lines();
            command_queue.push_back_unchecked(Command::Internal(InternalCommand::RedrawDisplay));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::commands::Command;
    use super::handle_event;
    use crate::{ExitStrategy, PagerState, minus_core::CommandQueue};
    use std::sync::{Arc, atomic::AtomicBool};

    const TEST_STR: &str = "This is some sample text";

    // Tests for event emitting functions of Pager
    #[test]
    #[cfg(any(feature = "dynamic_output", feature = "static_output"))]
    fn set_data() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetData(TEST_STR.to_string());
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            ev,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );

        assert_eq!(ps.screen.formatted_lines, vec![TEST_STR.to_string()]);
    }

    #[test]
    fn append_str() {
        let mut ps = PagerState::new().unwrap();
        let ev1 = Command::AppendData(format!("{TEST_STR}\n"));
        let ev2 = Command::AppendData(TEST_STR.to_string());
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            ev1,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );
        handle_event(
            ev2,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(
            ps.screen.formatted_lines,
            vec![TEST_STR.to_string(), TEST_STR.to_string()]
        );
    }

    #[test]
    #[cfg(any(feature = "dynamic_output", feature = "static_output"))]
    fn set_prompt() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetPrompt(TEST_STR.to_string());
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            ev,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(ps.prompt, TEST_STR.to_string());
    }

    #[test]
    #[cfg(any(feature = "dynamic_output", feature = "static_output"))]
    fn send_message() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SendMessage(TEST_STR.to_string());
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            ev,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(ps.message.unwrap(), TEST_STR.to_string());
    }

    #[test]
    #[cfg(feature = "static_output")]
    fn set_run_no_overflow() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetRunNoOverflow(false);
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            ev,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );
        assert!(!ps.run_no_overflow);
    }

    #[test]
    fn set_exit_strategy() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::SetExitStrategy(ExitStrategy::PagerQuit);
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            ev,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(ps.exit_strategy, ExitStrategy::PagerQuit);
    }

    #[test]
    fn add_exit_callback() {
        let mut ps = PagerState::new().unwrap();
        let ev = Command::AddExitCallback(Box::new(|| println!("Hello World")));
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            ev,
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(ps.exit_callbacks.len(), 1);
    }
}
