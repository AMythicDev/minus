//! Provides the [`InputHandler`] trait, which can be used
//! to customize the default keybindings of minus

use crossterm::{
    event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind},
    terminal,
};

#[cfg(feature = "search")]
use super::utils::SearchMode;
use crate::LineNumbers;

/// Events handled by the `minus` pager.
#[derive(Debug, Copy, Clone, PartialEq)]
#[allow(clippy::module_name_repetitions)]
pub enum InputEvent {
    /// `Ctrl+C` or `Q`, exits the application.
    Exit,
    /// The terminal was resized. Contains the new number of rows.
    UpdateTermArea(usize, usize),
    /// `Up` or `Down` was pressed. Contains the new value for the upper mark.
    /// Also sent by `g` or `G`, which behave like Vim: jump to top or bottom.
    UpdateUpperMark(usize),
    /// `Ctrl+L`, inverts the line number display. Contains the new value.
    UpdateLineNumber(LineNumbers),
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
/// use minus::{input::{InputEvent, InputHandler}, LineNumbers, Pager};
#[cfg_attr(feature = "search", doc = "use minus::SearchMode;")]
/// use crossterm::event::{Event, KeyEvent, KeyCode, KeyModifiers};
///
/// struct CustomInputHandler;
/// impl InputHandler for CustomInputHandler {
///     fn handle_input(
///         &self,
///         ev: Event,
///         upper_mark: usize,
///         // A `search_mode` parameter is available, if `search` feature is enabled
#[cfg_attr(feature = "search", doc = "        search_mode: SearchMode,")]
///         ln: LineNumbers,
///         rows: usize
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
///                       (upper_mark.saturating_sub(1))),
///                 _ => None
///         }
///     }
/// }
///
/// let pager = Pager::new().set_input_handler(
///                 Box::new(CustomInputHandler)
///             );
/// ```
#[allow(clippy::module_name_repetitions)]
pub trait InputHandler {
    fn handle_input(
        &self,
        ev: Event,
        upper_mark: usize,
        #[cfg(feature = "search")] search_mode: SearchMode,
        ln: LineNumbers,
        rows: usize,
    ) -> Option<InputEvent>;
}

/// The default keybindings in `minus`. These can be overriden by
/// making a custom input handler struct and implementing the [`InputHandler`] trait
pub struct DefaultInputHandler;

impl InputHandler for DefaultInputHandler {
    #[allow(clippy::too_many_lines)]
    fn handle_input(
        &self,
        ev: Event,
        upper_mark: usize,
        #[cfg(feature = "search")] search_mode: SearchMode,
        ln: LineNumbers,
        rows: usize,
    ) -> Option<InputEvent> {
        match ev {
            // Scroll up by one.
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Up || code == KeyCode::Char('k') => {
                Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(1)))
            }

            // Scroll down by one.
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
            }) if code == KeyCode::Down || code == KeyCode::Char('j') => {
                Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(1)))
            }

            // Scroll up by half screen height.
            Event::Key(KeyEvent {
                code: KeyCode::Char('u'),
                modifiers,
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = (terminal::size().ok()?.1 / 2) as usize;
                Some(InputEvent::UpdateUpperMark(
                    upper_mark.saturating_sub(half_screen),
                ))
            }
            // Scroll down by half screen height.
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers,
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = (terminal::size().ok()?.1 / 2) as usize;
                Some(InputEvent::UpdateUpperMark(
                    upper_mark.saturating_add(half_screen),
                ))
            }

            // Mouse scroll up/down
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollUp,
                ..
            }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(5))),
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::ScrollDown,
                ..
            }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(5))),
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
            }) => Some(InputEvent::UpdateUpperMark(usize::MAX)),

            // Page Up/Down
            Event::Key(KeyEvent {
                code: KeyCode::PageUp,
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::UpdateUpperMark(
                upper_mark.saturating_sub(rows - 1),
            )),
            Event::Key(KeyEvent {
                code: KeyCode::PageDown,
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::UpdateUpperMark(
                upper_mark.saturating_add(rows - 1),
            )),

            // Resize event from the terminal.
            Event::Resize(cols, rows) => {
                Some(InputEvent::UpdateTermArea(cols as usize, rows as usize))
            }
            // Switch line number display.
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::CONTROL,
            }) => Some(InputEvent::UpdateLineNumber(!ln)),
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
                if search_mode == SearchMode::Reverse {
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
                if search_mode == SearchMode::Reverse {
                    Some(InputEvent::NextMatch)
                } else {
                    Some(InputEvent::PrevMatch)
                }
            }
            _ => None,
        }
    }
}
