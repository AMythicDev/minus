//! Provides the [`handle_event`] function

use std::convert::TryInto;
use std::io::Write;
use std::sync::{Arc, atomic::AtomicBool};

#[cfg(feature = "search")]
use parking_lot::{Condvar, Mutex};

use super::CommandQueue;
use super::commands::{Command, IoCommand};
use super::utils::display::{self, AppendStyle};
use crate::ExitStrategy;
#[cfg(feature = "search")]
use crate::search;
use crate::{PagerState, error::MinusError, hooks::Hook, input::InputEvent};

/// Respond based on the type of command
///
/// It will match the type of event received and based on that, it can take actions like:-
/// - Mutating fields of [`PagerState`]
/// - Handle cleanup and exits
/// - Call search related functions
#[cfg_attr(not(feature = "search"), allow(unused_mut))]
#[allow(clippy::too_many_lines)]
// TODO: Remove it in next major release
#[allow(deprecated)]
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
            command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::Exit) => {
            p.run_hooks(Hook::PrePagerExit);
            p.exit();
            is_exited.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        Command::UserInput(InputEvent::UpdateUpperMark(um)) => {
            command_queue.push_back(Command::Io(IoCommand::SetUpperMark(um)));
        }
        Command::UserInput(InputEvent::UpdateLeftMark(lm)) if !p.screen.line_wrapping => {
            let padding = if p.line_numbers.is_on() {
                crate::minus_core::utils::digits(p.screen.line_count())
                    + crate::LineNumbers::EXTRA_PADDING
                    + 2
            } else {
                0
            };
            let max_scrollable = p.screen.get_max_line_length().saturating_add(padding);
            if lm.saturating_add(p.cols) > max_scrollable && lm > p.left_mark {
                return;
            }
            p.left_mark = lm;
            command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::StartSelection { x, y }) => {
            #[cfg(feature = "search")]
            if p.search_state.search_term.is_some() {
                return;
            }

            if let Some(selection) = p.selection_from_coordinates(x, y) {
                p.selection_anchor = Some(selection);
                p.selection = Some(selection);
                command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
            }
        }
        Command::UserInput(InputEvent::UpdateSelection { x, y }) => {
            #[cfg(feature = "search")]
            if p.search_state.search_term.is_some() {
                return;
            }

            if p.selection_anchor.is_none() {
                return;
            }

            let writable_rows = p.rows.saturating_sub(1);
            if writable_rows == 0 {
                return;
            }

            let row_count = p.screen.formatted_lines_count();
            let max_upper_mark = row_count.saturating_sub(writable_rows);
            let mut should_redraw = false;
            let mut selection_y = usize::from(y);

            if y == 0 {
                let next_upper_mark = p.upper_mark.saturating_sub(1);
                if next_upper_mark != p.upper_mark {
                    p.upper_mark = next_upper_mark;
                    should_redraw = true;
                }
                selection_y = 0;
            } else if selection_y >= writable_rows {
                let next_upper_mark = p.upper_mark.saturating_add(1).min(max_upper_mark);
                if next_upper_mark != p.upper_mark {
                    p.upper_mark = next_upper_mark;
                    should_redraw = true;
                }
                selection_y = writable_rows.saturating_sub(1);
            }

            #[allow(clippy::cast_possible_truncation)]
            if let Some(selection) = p.selection_from_coordinates(x, selection_y as u16)
                && p.selection != Some(selection)
            {
                p.selection = Some(selection);
                should_redraw = true;
            }

            if should_redraw {
                command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
            }
        }
        Command::UserInput(InputEvent::ClearSelection) => {
            if p.selection.is_some() || p.selection_anchor.is_some() {
                p.clear_selection();
                command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
            }
        }
        Command::UserInput(InputEvent::RestorePrompt) => {
            // Set the message to None and new messages to false as all messages have been shown
            p.message = None;
            p.format_prompt();
            command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
        }
        Command::UserInput(InputEvent::UpdateTermArea(c, r)) => {
            p.rows = r;
            p.cols = c;
            p.format_lines();
            // Readjust the text wrapping for the new number of columns
            command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::UpdateLineNumber(l)) => {
            p.line_numbers = l;
            p.format_lines();
            command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
        }
        Command::UserInput(InputEvent::Number(n)) => {
            p.prefix_num.push(n);
            p.format_prompt();
            command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
        }
        #[cfg(feature = "search")]
        Command::UserInput(InputEvent::Search(m)) => {
            p.search_mode = m;
            p.search_state.search_mode = m;
            p.search_state.search_mark = 0;
            command_queue.push_back(Command::Io(IoCommand::FetchSearchQuery));
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
                command_queue.push_back(Command::Io(IoCommand::SetUpperMark(upper_mark)));
                p.format_prompt();
                command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
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
                    command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
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
                command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
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
                    command_queue.push_back(Command::Io(IoCommand::SetUpperMark(*y)));
                    p.format_prompt();
                    command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
                }
            }
        }

        Command::UserInput(InputEvent::HorizontalScroll(val)) => {
            p.screen.line_wrapping = val;
            p.format_lines();
            command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
        }

        Command::AppendData(text) => {
            let prev_unterminated = p.screen.unterminated;
            let prev_fmt_lines_count = p.screen.formatted_lines_count();
            let append_style = p.append_str(text.as_str());

            if append_style == AppendStyle::FullRedraw {
                command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
                return;
            }

            command_queue.push_back(Command::Io(IoCommand::DrawAppendedText(
                prev_unterminated,
                prev_fmt_lines_count,
                append_style,
            )));

            if p.follow_output {
                command_queue.push_back(Command::Io(IoCommand::SetUpperMark(
                    p.screen.formatted_lines_count(),
                )));
            }
        }

        Command::SetPrompt(ref text) | Command::SendMessage(ref text) => {
            if let Command::SetPrompt(_) = ev {
                p.prompt = text.clone();
            } else {
                p.message = Some(text.clone());
            }
            p.format_prompt();
            command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
        }
        Command::SetLineNumbers(ln) => {
            p.line_numbers = ln;
            p.format_lines();
            command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
        }
        Command::SetExitStrategy(es) => {
            p.hooks.remove_callback(Hook::PostPagerExit, 1);
            if es == ExitStrategy::ProcessQuit {
                p.hooks.add_callback(
                    Hook::PostPagerExit,
                    1,
                    Box::new(|_| {
                        std::process::exit(1);
                    }),
                );
            } else {
                p.hooks
                    .add_callback(Hook::PostPagerExit, 1, Box::new(|_| {}));
            }
        }
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
        Command::AddHook(hook, id, cb) => p.hooks.add_callback(hook, id, cb),
        Command::RemoveHook(hook, id) => {
            p.hooks.remove_callback(hook, id);
        }
        Command::ShowPrompt(show) => p.show_prompt = show,
        Command::FollowOutput(follow_output)
        | Command::UserInput(InputEvent::FollowOutput(follow_output)) => {
            p.follow_output = follow_output;
            command_queue.push_back(Command::UserInput(InputEvent::UpdateUpperMark(
                p.screen.formatted_lines_count(),
            )));
            p.format_prompt();
            command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
        }
        Command::UserInput(_) => {}
        Command::Io(_) => unreachable!(),
    }
}

#[cfg_attr(
    not(feature = "search"),
    allow(unused_variables),
    allow(clippy::needless_pass_by_ref_mut)
)]
pub fn handle_io_command(
    internal_command: IoCommand,
    mut out: &mut impl Write,
    p: &mut PagerState,
    command_queue: &mut CommandQueue,
    #[cfg(feature = "search")] user_input_active: &Arc<(Mutex<bool>, Condvar)>,
) -> Result<(), MinusError> {
    if p.running.lock().is_uninitialized() {
        return Ok(());
    }
    match internal_command {
        IoCommand::RedrawPrompt => {
            display::write_prompt(out, &p.displayed_prompt, p.rows.try_into().unwrap())?;
        }
        IoCommand::RedrawDisplay => {
            display::draw_full(&mut out, p)?;
        }
        IoCommand::SetUpperMark(mut um) => {
            display::draw_for_change(out, p, &mut um)?;
            let line_count = p.screen.formatted_lines_count();
            if um >= line_count.saturating_sub(p.rows.saturating_sub(1)) && line_count > p.rows {
                p.run_hooks(Hook::EofReached);
            }
            p.upper_mark = um;
        }
        IoCommand::DrawAppendedText(prev_unterminated, prev_fmt_lines_count, append_style) => {
            let AppendStyle::PartialUpdate(bounds) = append_style else {
                unreachable!();
            };
            let fmt_lines = p.render_rows_for_display(bounds.0, bounds.1);
            display::draw_append_text(
                out,
                p.rows,
                prev_unterminated,
                prev_fmt_lines_count,
                &fmt_lines,
            )?;
        }
        #[cfg(feature = "search")]
        IoCommand::FetchSearchQuery => {
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

            command_queue.push_back(Command::Io(IoCommand::RedrawPrompt));
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
                    command_queue.push_back(Command::SendMessage(
                        "Invalid regular expression. Press Enter".to_string(),
                    ));
                    return Ok(());
                }
                compiled_regex
            } else {
                return Ok(());
            };

            p.format_lines();
            command_queue.push_back(Command::Io(IoCommand::RedrawDisplay));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::commands::{Command, IoCommand};
    use super::handle_event;
    use crate::{PagerState, input::InputEvent, minus_core::CommandQueue, state::Selection};
    use std::fmt::Write;
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

    #[test]
    fn update_selection_scrolls_up_at_top_edge() {
        let mut ps = PagerState::new().unwrap();
        ps.rows = 5;
        ps.screen.orig_text = (0..10).fold(String::new(), |mut t, idx| {
            let _ = writeln!(t, "line {idx}");
            t
        });
        ps.format_lines();
        ps.upper_mark = 3;
        ps.selection_anchor = Some(Selection {
            absolute_row: 3,
            col: 0,
        });
        ps.selection = ps.selection_anchor;
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            Command::UserInput(InputEvent::UpdateSelection { x: 0, y: 0 }),
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );

        assert_eq!(ps.upper_mark, 2);
        assert_eq!(
            ps.selection,
            Some(Selection {
                absolute_row: 2,
                col: 0,
            })
        );
        assert_eq!(
            command_queue.pop_front(),
            Some(Command::Io(IoCommand::RedrawDisplay))
        );
    }

    #[test]
    fn update_selection_scrolls_down_at_bottom_edge() {
        let mut ps = PagerState::new().unwrap();
        ps.rows = 5;
        ps.screen.orig_text = (0..10).fold(String::new(), |mut t, idx| {
            let _ = writeln!(t, "line {idx}");
            t
        });
        ps.format_lines();
        ps.upper_mark = 3;
        ps.selection_anchor = Some(Selection {
            absolute_row: 3,
            col: 0,
        });
        ps.selection = ps.selection_anchor;
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            Command::UserInput(InputEvent::UpdateSelection { x: 0, y: 4 }),
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );

        assert_eq!(ps.upper_mark, 4);
        assert_eq!(
            ps.selection,
            Some(Selection {
                absolute_row: 7,
                col: 0,
            })
        );
        assert_eq!(
            command_queue.pop_front(),
            Some(Command::Io(IoCommand::RedrawDisplay))
        );
    }

    #[test]
    fn update_selection_clamps_scroll_at_bottom_bound() {
        let mut ps = PagerState::new().unwrap();
        ps.rows = 5;
        ps.screen.orig_text = (0..10).fold(String::new(), |mut t, idx| {
            let _ = writeln!(t, "line {idx}");
            t
        });
        ps.format_lines();
        ps.upper_mark = 2;
        ps.selection_anchor = Some(Selection {
            absolute_row: 2,
            col: 0,
        });
        ps.selection = ps.selection_anchor;
        let mut command_queue = CommandQueue::new_zero();

        handle_event(
            Command::UserInput(InputEvent::UpdateSelection { x: 0, y: 10 }),
            &mut ps,
            &mut command_queue,
            &Arc::new(AtomicBool::new(false)),
        );

        assert_eq!(ps.upper_mark, 2);
        assert_eq!(
            ps.selection,
            Some(Selection {
                absolute_row: 5,
                col: 0,
            })
        );
        assert_eq!(
            command_queue.pop_front(),
            Some(Command::Io(IoCommand::RedrawDisplay))
        );
    }
}
