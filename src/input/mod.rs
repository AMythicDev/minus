//! Manage keyboard/mouse-bindings while running `minus`.
//!
//! > **Terminology in this module**: We will call any keyboard/mouse event from the terminal as a **binding**
//! and its associated predefined action as **callback**.
//!
//! There are two ways to define binding in minus as you will see below.
//!
//! # Newer (Recommended) Method
//! ## Description
//! This method offers a much improved and ergonomic API for defining bindings and callbacks.
//! You use the [HashedEventRegister] for registering bindings and their associated callback.
//! It provides functions like [add_key_events](HashedEventRegister::add_key_events) and
//! [add_mouse_events](HashedEventRegister::add_mouse_events) which take `&[&str]` as its first
//! argument and a callback `cb` as its second argument and maps all `&str` in the `&[&str]` to
//! same callback function `cb`. Each `&str` of the `&[&str]` contains a description of the
//! key/mouse binding needed to activate it. For example `c-c` means pressing a `Ctrl+c` on the
//! keyboard. See [Writing Binding Descriptions](#writing-binding-descriptions) to know more on
//! writing these descriptions.
//
//! ## Example
//! ```
//! use minus::input::{InputEvent, HashedEventRegister, crossterm_event::Event};
//!
//! let mut input_register = HashedEventRegister::default();
//!
//! input_register.add_key_events(&["down"], |_, ps| {
//!     InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(1))
//! });
//!
//! input_register.add_mouse_events(&["scroll:up"], |_, ps| {
//!     InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(5))
//! });
//!
//! input_register.add_resize_event(|ev, _| {
//!     let (cols, rows) = if let Event::Resize(cols, rows) = ev {
//!         (cols, rows)
//!     } else {
//!        unreachable!();
//!     };
//!     InputEvent::UpdateTermArea(cols as usize, rows as usize)
//! });
//! ```
//!
//! ## Writing Binding Descriptions
//! ### Defining Keybindings
//! The general syntax for defining keybindings is `[MODIFIER]-[MODIFIER]-[MODIFIER]-{SINGLE KEY}`
//!
//! `MODIFIER`s include or or more of the `Ctrl` `Alt` and `Shift` keys. They are writeen with
//! the shorthands `c`, `m` and `s` respectively.
//!
//! `SINGLE CHAR` includes any key on the keyboard which is not a modifier like `a`, `z`, `1`, `F1`
//! or `enter`. Each of these pieces are separated by a `-`.
//!
//! Here are some examples
//!
//! | Key Input    | Mean ing                                   |
//! |--------------|--------------------------------------------|
//! | `a`          | A literal `a`                              |
//! | `Z`          | A `Z`. Matched only when a caps lock is on |
//! | `c-q`        | `Ctrl+q`                                   |
//! | `enter`      | `ENTER` key                                |
//! | `c-m-pageup` | `Ctrl+Alt+PageUp`                          |
//! | `s-2`        | `Shift+2`                                  |
//! | `backspace`  | `Backspace` Key                            |
//! | `left`       | `Left Arrow` key                           |
//!
//! ### Defining Mouse Bindings
//!
//! The general syntax for defining keybindings is `[MODIFIER]-[MODIFIER]-[MODIFIER]-{MOUSE ACTION}`
//!
//! `MODIFIER`s include or or more of the `Ctrl` `Alt` and `Shift` keys which are pressed along
//! with the mouse action. They are writeen with the shorthands `c`, `m` and `s` respectively.
//!
//! `MOUSE ACTION` includes actions like pressing down the left mouse button or taking up the right
//! mouse button. It also includes scrolling up/down or pressing the middle click.
//!
//! Here are some examples
//!
//! | Key Input     | Mean ing                                   |
//! |---------------|--------------------------------------------|
//! | `left:up`     | Releasing the left mouse button            |
//! | `right:down`  | Pressing the right mouse button            |
//! | `c-mid:down`  | Middle click in pressed along with Ctrl key|
//! | `m-scroll:up` | Scrolled down while pressing the Alt key   |
//!
//! **NOTE:** Although minus's description parser can correctly parse almost all if not all the
//!   events that you can possibly register, not all of them are correctly registered by crossterm
//!   itself. For example minus corrctly parses `c-s-h` as  `ctrl+shift-h` but crossterm
//!   categorically recognizes it as `ctrl+h` when reading events from the terminal.
//!
//! # Legacy method
//! This method relies heavily on the [`InputClassifier`] trait and end-applications were needed to
//! manually copy the [default definitions](DefaultInputClassifier) and make the required
//! modifications yourself in this method. This lead to very messy and error-prone system for
//! defining bindings and also required application authors to bring in the the underlying
//! [crossterm](https://docs.rs/crossterm/latest) crate to define the events.
//!
//! ## Example
//! ```
//! use minus::{input::{InputEvent, InputClassifier}, Pager, PagerState};
//! use crossterm::event::{Event, KeyEvent, KeyCode, KeyModifiers};
//!
//! struct CustomInputClassifier;
//! impl InputClassifier for CustomInputClassifier {
//!     fn classify_input(
//!         &self,
//!         ev: Event,
//!         ps: &PagerState
//!     ) -> Option<InputEvent> {
//!             match ev {
//!                 Event::Key(KeyEvent {
//!                     code: KeyCode::Up,
//!                     modifiers: KeyModifiers::NONE,
//!                     ..
//!                 })
//!                 | Event::Key(KeyEvent {
//!                     code: KeyCode::Char('j'),
//!                     modifiers: KeyModifiers::NONE,
//!                     ..
//!                 }) => Some(InputEvent::UpdateUpperMark
//!                       (ps.upper_mark.saturating_sub(1))),
//!                 _ => None
//!         }
//!     }
//! }
//!
//! let mut pager = Pager::new();
//! pager.set_input_classifier(
//!                 Box::new(CustomInputClassifier)
//!             );
//! ```
//!
//! **NOTE:** Although you can define almost every combination of bindings that crossterm supports,
//!   not all of them are correctly registered by crossterm itself. For example you can define
//!   ```text
//!   Event::Key(KeyEvent {
//!       code: KeyCode::Char(`h`),
//!       modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
//!       ..
//!   })
//!   ```
//!   but crossterm will not match to it as crossterm
//!   recognizes a `ctrl+shift+h` as `ctrl+h` when reading events from the terminal.
//!
//! # Custom Actions on User Events
//!
//! Sometimes you want to execute arbitrary code when a key/mouse action is pressed like fetching
//! more data from a server but not necessarily sending it to minus. In these types of scenarios,
//! the [InputEvent::Ignore] is most likely your true friend. When this is returned by a callback
//! function, minus will execute your code but not do anything special for the event on its part.
//! ```no_test
//! input_register.add_key_events(&["f"], |_, ps| {
//!     fetch_data_from_server(...);
//!     InputEvent::Ignore
//! });
//! ```
//! It can be used with the legacy method too.
//! ```no_test
//! struct CustomInputClassifier;
//! impl InputClassifier for CustomInputClassifier {
//!     fn classify_input(
//!         &self,
//!         ev: Event,
//!         ps: &PagerState
//!     ) -> Option<InputEvent> {
//!             match ev {
//!                 Event::Key(KeyEvent {
//!                     code: KeyCode::Char('f'),
//!                     modifiers: KeyModifiers::NONE,
//!                     ..
//!                 }) => {
//!                     fetch_data_from_server(...);
//!                     InputEvent::Ignore
//!                 },
//!                 _ => None
//!         }
//!     }
//! }
//! ```

pub(crate) mod definitions;
pub(crate) mod hashed_event_register;

pub use crossterm::event as crossterm_event;

#[cfg(feature = "search")]
use crate::search::SearchMode;
use crate::{LineNumbers, PagerState};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
pub use hashed_event_register::HashedEventRegister;

/// Events handled by the `minus` pager.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[allow(clippy::module_name_repetitions)]
#[non_exhaustive]
pub enum InputEvent {
    /// `Ctrl+C` or `Q`, exits the application.
    Exit,
    /// The terminal was resized. Contains the new number of rows.
    UpdateTermArea(usize, usize),
    /// Sent by movement keys like `Up` `Down`, `PageUp`, 'PageDown', 'g', `G` etc.
    /// Contains the new value for the upper mark.
    UpdateUpperMark(usize),
    /// `Ctrl+L`, inverts the line number display. Contains the new value.
    UpdateLineNumber(LineNumbers),
    /// A number key has been pressed. This inner value is stored as a `char`.
    /// The input loop will append this number to its `count` string variable
    Number(char),
    /// Restore the original prompt
    RestorePrompt,
    /// Whether to allow Horizontal scrolling
    HorizontalScroll(bool),
    /// Sets the left mark of Horizontal scrolling
    ///
    /// Sent by keys like `l`, `h`, `right`, `left` etc.
    UpdateLeftMark(usize),
    /// Tells the event hadler to not do anything for this event
    ///
    /// This is extremely useful when you want to execute arbitrary code on events without
    /// necessarily asking the event handler to do anything special for this event. See [Custom
    /// Actions on User Events](./index.html#custom-actions-on-user-events).
    Ignore,
    /// `/`, Searching for certain pattern of text
    #[cfg(feature = "search")]
    Search(SearchMode),
    /// Get to the next match in forward mode
    ///
    /// **WARNING: This has been deprecated in favour of `MoveToNextMatch`. This will likely be
    /// removed in the next major release.**
    #[cfg(feature = "search")]
    NextMatch,
    /// Get to the previous match in forward mode
    ///
    /// **WARNING: This has been deprecated in favour of `MoveToPrevMatch`. This will likely be
    /// removed in the next major release.**
    #[cfg(feature = "search")]
    PrevMatch,
    /// Move to the next nth match in the given direction
    #[cfg(feature = "search")]
    MoveToNextMatch(usize),
    /// Move to the previous nth match in the given direction
    #[cfg(feature = "search")]
    MoveToPrevMatch(usize),
    /// Control follow mode.
    ///
    /// When set to true, minus ensures that the user's screen always follows the end part of the
    /// output. By default it is turned off.
    ///
    /// This is similar to [Pager::follow_output](crate::pager::Pager::follow_output) except that
    /// this is used to control it from the user's side.
    FollowOutput(bool),
}

/// Classifies the input and returns the appropriate [`InputEvent`]
///
/// If you are using the newer method for input definition, you don't need to take care of this.
///
/// If you are using the legacy method, see the sources of [`DefaultInputClassifier`] on how to
/// inplement this trait.
#[allow(clippy::module_name_repetitions)]
pub trait InputClassifier {
    fn classify_input(&self, ev: Event, ps: &PagerState) -> Option<InputEvent>;
}

/// Insert the default set of actions into the [`HashedEventRegister`]
#[allow(clippy::too_many_lines)]
pub fn generate_default_bindings<S>(map: &mut HashedEventRegister<S>)
where
    S: std::hash::BuildHasher,
{
    map.add_key_events(&["q", "c-c"], |_, _| InputEvent::Exit);

    map.add_key_events(&["up", "k"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(position))
    });
    map.add_key_events(&["down", "j"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position))
    });
    map.add_key_events(&["c-f"], |_, ps| {
        InputEvent::FollowOutput(!ps.follow_output)
    });
    map.add_key_events(&["enter"], |_, ps| {
        if ps.message.is_some() {
            InputEvent::RestorePrompt
        } else {
            let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
            InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position))
        }
    });
    map.add_key_events(&["u", "c-u"], |_, ps| {
        let half_screen = ps.rows / 2;
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(half_screen))
    });
    map.add_key_events(&["d", "c-d"], |_, ps| {
        let half_screen = ps.rows / 2;
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(half_screen))
    });
    map.add_key_events(&["g"], |_, _| InputEvent::UpdateUpperMark(0));

    map.add_key_events(&["s-g", "G"], |_, ps| {
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
        // Get the exact row number where first row of this line is placed in
        // [`PagerState::formatted_lines`] and jump to that location.If the line number does not
        // exist, directly jump to the bottom of text.
        let row_to_go = *ps
            .lines_to_row_map
            .get(position)
            .unwrap_or(&(usize::MAX - 1));
        InputEvent::UpdateUpperMark(row_to_go)
    });
    map.add_key_events(&["pageup"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(ps.rows - 1))
    });
    map.add_key_events(&["pagedown", "space"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(ps.rows - 1))
    });
    map.add_key_events(&["c-l"], |_, ps| {
        InputEvent::UpdateLineNumber(!ps.line_numbers)
    });
    #[cfg(feature = "search")]
    {
        map.add_key_events(&["/"], |_, _| InputEvent::Search(SearchMode::Forward));
        map.add_key_events(&["?"], |_, _| InputEvent::Search(SearchMode::Reverse));
        map.add_key_events(&["n"], |_, ps| {
            let position = ps.prefix_num.parse::<usize>().unwrap_or(1);

            if ps.search_state.search_mode == SearchMode::Forward {
                InputEvent::MoveToNextMatch(position)
            } else if ps.search_state.search_mode == SearchMode::Reverse {
                InputEvent::MoveToPrevMatch(position)
            } else {
                InputEvent::Ignore
            }
        });
        map.add_key_events(&["p"], |_, ps| {
            let position = ps.prefix_num.parse::<usize>().unwrap_or(1);

            if ps.search_state.search_mode == SearchMode::Forward {
                InputEvent::MoveToPrevMatch(position)
            } else if ps.search_state.search_mode == SearchMode::Reverse {
                InputEvent::MoveToNextMatch(position)
            } else {
                InputEvent::Ignore
            }
        });
    }

    map.add_mouse_events(&["scroll:up"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(5))
    });
    map.add_mouse_events(&["scroll:down"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(5))
    });

    map.add_key_events(&["c-s-h", "c-h"], |_, ps| {
        InputEvent::HorizontalScroll(!ps.screen.line_wrapping)
    });
    map.add_key_events(&["h", "left"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateLeftMark(ps.left_mark.saturating_sub(position))
    });
    map.add_key_events(&["l", "right"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateLeftMark(ps.left_mark.saturating_add(position))
    });
    // TODO: Add keybindings for left right scrolling

    map.add_resize_event(|ev, _| {
        let Event::Resize(cols, rows) = ev else {
            unreachable!();
        };
        InputEvent::UpdateTermArea(cols as usize, rows as usize)
    });

    map.insert_wild_event_matcher(|ev, _| {
        if let Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            ..
        }) = ev
        {
            if c.is_ascii_digit() {
                InputEvent::Number(c)
            } else {
                InputEvent::Ignore
            }
        } else {
            InputEvent::Ignore
        }
    });
}

/// The default set of input definitions
///
/// **This is kept only for legacy purposes and may not be well updated with all the latest changes**
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
                ..
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
                ..
            }) if code == KeyCode::Down || code == KeyCode::Char('j') => {
                let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                Some(InputEvent::UpdateUpperMark(
                    ps.upper_mark.saturating_add(position),
                ))
            }

            // Toggle output following
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::CONTROL,
                ..
            }) if code == KeyCode::Char('f') => Some(InputEvent::FollowOutput(!ps.follow_output)),

            // For number keys
            Event::Key(KeyEvent {
                code: KeyCode::Char(c),
                modifiers: KeyModifiers::NONE,
                ..
            }) if c.is_ascii_digit() => Some(InputEvent::Number(c)),

            // Enter key
            Event::Key(KeyEvent {
                code: KeyCode::Enter,
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                if ps.message.is_some() {
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
                ..
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = ps.rows / 2;
                Some(InputEvent::UpdateUpperMark(
                    ps.upper_mark.saturating_sub(half_screen),
                ))
            }
            // Scroll down by half screen height.
            Event::Key(KeyEvent {
                code: KeyCode::Char('d'),
                modifiers,
                ..
            }) if modifiers == KeyModifiers::CONTROL || modifiers == KeyModifiers::NONE => {
                let half_screen = ps.rows / 2;
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
                ..
            }) => Some(InputEvent::UpdateUpperMark(0)),
            // Go to bottom.
            Event::Key(KeyEvent {
                code: KeyCode::Char('g'),
                modifiers: KeyModifiers::SHIFT,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('G'),
                modifiers: KeyModifiers::SHIFT,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('G'),
                modifiers: KeyModifiers::NONE,
                ..
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
                ..
            }) => Some(InputEvent::UpdateUpperMark(
                ps.upper_mark.saturating_sub(ps.rows - 1),
            )),
            Event::Key(KeyEvent {
                code: c,
                modifiers: KeyModifiers::NONE,
                ..
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
                ..
            }) => Some(InputEvent::UpdateLineNumber(!ps.line_numbers)),

            // Quit.
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::NONE,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => Some(InputEvent::Exit),

            // Horizontal scrolling
            Event::Key(KeyEvent {
                code: KeyCode::Char('h'),
                modifiers,
                ..
            }) if modifiers == KeyModifiers::CONTROL.intersection(KeyModifiers::SHIFT) => {
                Some(InputEvent::HorizontalScroll(!ps.screen.line_wrapping))
            }

            Event::Key(KeyEvent {
                code: KeyCode::Char('h'),
                modifiers: KeyModifiers::NONE,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Left,
                modifiers: KeyModifiers::NONE,
                ..
            }) => Some(InputEvent::UpdateLeftMark(ps.left_mark.saturating_sub(1))),
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                modifiers: KeyModifiers::NONE,
                ..
            })
            | Event::Key(KeyEvent {
                code: KeyCode::Right,
                modifiers: KeyModifiers::NONE,
                ..
            }) => Some(InputEvent::UpdateLeftMark(ps.left_mark.saturating_add(1))),

            // Search
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('/'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => Some(InputEvent::Search(SearchMode::Forward)),
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('?'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => Some(InputEvent::Search(SearchMode::Reverse)),
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('n'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                if ps.search_state.search_mode == SearchMode::Reverse {
                    Some(InputEvent::MoveToPrevMatch(position))
                } else {
                    Some(InputEvent::MoveToNextMatch(position))
                }
            }
            #[cfg(feature = "search")]
            Event::Key(KeyEvent {
                code: KeyCode::Char('p'),
                modifiers: KeyModifiers::NONE,
                ..
            }) => {
                let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
                if ps.search_state.search_mode == SearchMode::Reverse {
                    Some(InputEvent::MoveToNextMatch(position))
                } else {
                    Some(InputEvent::MoveToPrevMatch(position))
                }
            }
            _ => None,
        }
    }
}
#[cfg(test)]
mod tests;
