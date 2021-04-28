use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use super::utils::InputEvent;
#[cfg(feature = "search")]
use super::utils::SearchMode;
use crate::LineNumbers;

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
/// use minus::{InputEvent, InputHandler, LineNumbers, Pager};
/// use crossterm::event::{Event, KeyEvent, KeyCode, KeyModifiers};
///
/// struct CustomInputHandler;
/// impl InputHandler for CustomInputHandler {
///     fn handle_input(
///         &self,
///         ev: Event,
///         upper_mark: usize,
///         // A search parameter is available, if `search` feature is enabled
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
/// fn main() {
///     let pager = Pager::new().set_input_handler(
///                     Box::new(CustomInputHandler)
///                 );
/// }
/// ```
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
                code: KeyCode::Up,
                modifiers: KeyModifiers::NONE,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('j'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_sub(1))),

            // Scroll down by one.
            Event::Key(KeyEvent {
                code: KeyCode::Down,
                modifiers: KeyModifiers::NONE,
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('k'),
                modifiers: KeyModifiers::NONE,
            }) => Some(InputEvent::UpdateUpperMark(upper_mark.saturating_add(1))),

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
            Event::Resize(_, height) => Some(InputEvent::UpdateRows(height as usize)),
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
