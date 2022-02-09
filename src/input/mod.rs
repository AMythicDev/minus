//! Provides the [`InputClassifier`] trait, which can be used
//! to customize the default keybindings of minus

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

#[cfg(feature = "search")]
use crate::minus_core::search::SearchMode;
use crate::{LineNumbers, PagerState};

/// Events handled by the `minus` pager.
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub enum InputEvent {
    /// `Ctrl+C` or `Q`, exits the application.
    Exit,
    /// The terminal was resized. Contains the new number of rows.
    UpdateTermArea(usize, usize),
    /// Sent by movement keys like `Up` `Down`, `PageUp`, 'PageDown', 'g', `G` etc. Contains the new value for the upper mark.
    UpdateUpperMark(usize),
    /// `Ctrl+L`, inverts the line number display. Contains the new value.
    UpdateLineNumber(LineNumbers),
    /// A number key has been pressed. This inner value is stored as a `char`.
    /// The input loop will append this number to its `count` string variable
    Number(char),
    /// Restore the original prompt
    RestorePrompt,
    /// `/`, Searching for certain pattern of text
    #[cfg(feature = "search")]
    Search(SearchMode),
    /// Get to the next match in forward mode
    #[cfg(feature = "search")]
    NextMatch,
    /// Get to the previous match in forward mode
    #[cfg(feature = "search")]
    PrevMatch,
}

/// Define custom keybindings
///
/// This trait can help define custom keybindings in case
/// the downsteam applications aren't satisfied with the
/// defaults
///
/// **Please do note that, in order to match the keybindings,
/// you need to directly work with the underlying [`crossterm`]
/// crate**
///
/// # Example
/// ```
/// use minus::{input::{InputEvent, InputClassifier}, LineNumbers, Pager, PagerState};
#[cfg_attr(feature = "search", doc = "use minus::SearchMode;")]
/// use crossterm::event::{Event, KeyEvent, KeyCode, KeyModifiers};
///
/// struct CustomInputClassifier;
/// impl InputClassifier for CustomInputClassifier {
///     fn classify_input(
///         &self,
///         ev: Event,
///         ps: &PagerState
///     ) -> Option<InputEvent> {
///             match ev {
///                 Event::Key(KeyEvent {
///                     code: KeyCode::Up,
///                     modifiers: KeyModifiers::NONE,
///                 })
///                 | Event::Key(KeyEvent {
///                     code: KeyCode::Char('j'),
///                     modifiers: KeyModifiers::NONE,
///                 }) => Some(InputEvent::UpdateUpperMark
///                       (ps.upper_mark.saturating_sub(1))),
///                 _ => None
///         }
///     }
/// }
///
/// let mut pager = Pager::new();
/// pager.set_input_classifier(
///                 Box::new(CustomInputClassifier)
///             );
/// ```
#[allow(clippy::module_name_repetitions)]
pub trait InputClassifier {
    fn classify_input(&self, ev: Event, ps: &PagerState) -> Option<InputEvent>;
}

/// The default keybindings in `minus`. These can be overriden by
/// making a custom input handler struct and implementing the [`InputClassifier`] trait
pub struct DefaultInputClassifier;

impl InputClassifier for DefaultInputClassifier {
    #[allow(clippy::too_many_lines)]
    fn classify_input(&self, ev: Event, ps: &PagerState) -> Option<InputEvent> {
        #[allow(clippy::unnested_or_patterns)]
        match ev {
            // Scroll up by one.
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Up || code == KeyCode::Char('k') => {
                let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                Some(InputEvent::UpdateUpperMark(
                    ps.upper_mark.saturating_sub(position),
                ))
            }

            // Scroll down by one.
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Down || code == KeyCode::Char('j') => {
                let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                Some(InputEvent::UpdateUpperMark(
                    ps.upper_mark.saturating_add(position),
                ))
            }

            // For number keys
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
            }) if c.is_digit(10) => Some(InputEvent::Number(c)),

            // Enter key
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
            }) => {
                if ps.message.1 {
                    Some(InputEvent::RestorePrompt)
                } else {
                    let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                    Some(InputEvent::UpdateUpperMark(
                        ps.upper_mark.saturating_add(position),
                    ))
                }
            }

            // Scroll up by half screen height.
            Event::Key(KeyEvent {
                code: KeyCode::Char('u'),
                modifiers,
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = (ps.rows / 2) as usize;
                Some(InputEvent::UpdateUpperMark(
                    ps.upper_mark.saturating_sub(half_screen),
                ))
            }
            // Scroll down by half screen height.
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers,
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = (ps.rows / 2) as usize;
                Some(InputEvent::UpdateUpperMark(
                    ps.upper_mark.saturating_add(half_screen),
                ))
            }

            // Mouse scroll up/down
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                ..
            }) => Some(InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(5))),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                ..
            }) => Some(InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(5))),
            // Go to top.
            Event::Key(KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::UpdateUpperMark(0)),
            // Go to bottom.
            Event::Key(KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::SHIFT,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('G'),
                modifiers: KeyModifiers::SHIFT,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('G'),
                modifiers: KeyModifiers::NONE,
            }) => {
                let mut position = ps
                    .prefix_num
                    .parse::<usize>()
                    .unwrap_or(usize::MAX)
                    // Reduce 1 here, because line numbering starts from 1
                    // while upper_mark starts from 0
                    .saturating_sub(1);
                if position == 0 {
                    position = usize::MAX;
                }
                Some(InputEvent::UpdateUpperMark(position))
            }

            // Page Up/Down
            Event::Key(KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::UpdateUpperMark(
                ps.upper_mark.saturating_sub(ps.rows - 1),
            )),
            Event::Key(KeyEvent {
                code: c,
                modifiers: KeyModifiers::NONE,
            }) if c == KeyCode::PageDown || c == KeyCode::Char(' ') => Some(
                InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(ps.rows - 1)),
            ),

            // Resize event from the terminal.
            Event::Resize(cols, rows) => {
                Some(InputEvent::UpdateTermArea(cols as usize, rows as usize))
            }
            // Switch line number display.
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::CONTROL,
            }) => Some(InputEvent::UpdateLineNumber(!ps.line_numbers)),
            // Quit.
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
            }) => Some(InputEvent::Exit),
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::Search(SearchMode::Forward)),
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::Search(SearchMode::Reverse)),
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if ps.search_mode == SearchMode::Reverse {
                    Some(InputEvent::PrevMatch)
                } else {
                    Some(InputEvent::NextMatch)
                }
            }
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
            }) => {
                if ps.search_mode == SearchMode::Reverse {
                    Some(InputEvent::NextMatch)
                } else {
                    Some(InputEvent::PrevMatch)
                }
            }
            _ => None,
        }
    }
}
#[cfg(test)]
mod tests;
