//! Manage keyboard/mouse-bindings while running `minus`.
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

pub(crate) mod definitions;
pub(crate) mod hashed_event_register;

pub use crossterm::event as crossterm_event;
pub use definitions::{keydefs::parse_key_event, mousedefs::parse_mouse_event};

#[cfg(feature = "search")]
use crate::search::SearchMode;
use crate::{LineNumbers, PagerState};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
pub(crate) use hashed_event_register::HashedEventRegister;

pub type InputEventBoxed = Box<dyn Fn(Event, &PagerState) -> InputEvent + Send + Sync + 'static>;

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

/// Insert the default set of actions into the [`HashedEventRegister`]
#[allow(clippy::too_many_lines)]
pub(crate) fn generate_default_bindings(map: &mut HashedEventRegister) {
    map.map_keys(&["q", "c-c"], |_, _| InputEvent::Exit);

    map.map_keys(&["up", "k"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(position))
    });
    map.map_keys(&["down", "j"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position))
    });
    map.map_keys(&["c-f"], |_, ps| {
        InputEvent::FollowOutput(!ps.follow_output)
    });
    map.map_keys(&["enter"], |_, ps| {
        if ps.message.is_some() {
            InputEvent::RestorePrompt
        } else {
            let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
            InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(position))
        }
    });
    map.map_keys(&["u", "c-u"], |_, ps| {
        let half_screen = ps.rows / 2;
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(half_screen))
    });
    map.map_keys(&["d", "c-d"], |_, ps| {
        let half_screen = ps.rows / 2;
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(half_screen))
    });
    map.map_keys(&["g", "home"], |_, _| InputEvent::UpdateUpperMark(0));

    map.map_keys(&["s-g", "G"], |_, ps| {
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
    map.map_keys(&["pageup"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(ps.rows - 1))
    });
    map.map_keys(&["pagedown", "space"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(ps.rows - 1))
    });
    map.map_keys(&["c-l"], |_, ps| {
        InputEvent::UpdateLineNumber(!ps.line_numbers)
    });
    map.map_keys(&["end"], |_, _| InputEvent::UpdateUpperMark(usize::MAX - 1));
    #[cfg(feature = "search")]
    {
        map.map_keys(&["/"], |_, _| InputEvent::Search(SearchMode::Forward));
        map.map_keys(&["?"], |_, _| InputEvent::Search(SearchMode::Reverse));
        map.map_keys(&["n"], |_, ps| {
            let position = ps.prefix_num.parse::<usize>().unwrap_or(1);

            if ps.search_state.search_mode == SearchMode::Forward {
                InputEvent::MoveToNextMatch(position)
            } else if ps.search_state.search_mode == SearchMode::Reverse {
                InputEvent::MoveToPrevMatch(position)
            } else {
                InputEvent::Ignore
            }
        });
        map.map_keys(&["p", "s-n"], |_, ps| {
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

    map.map_mouse(&["scroll:up"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_sub(5))
    });
    map.map_mouse(&["scroll:down"], |_, ps| {
        InputEvent::UpdateUpperMark(ps.upper_mark.saturating_add(5))
    });

    map.map_keys(&["c-s-h", "c-h"], |_, ps| {
        InputEvent::HorizontalScroll(!ps.screen.line_wrapping)
    });
    map.map_keys(&["h", "left"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateLeftMark(ps.left_mark.saturating_sub(position))
    });
    map.map_keys(&["l", "right"], |_, ps| {
        let position = ps.prefix_num.parse::<usize>().unwrap_or(1);
        InputEvent::UpdateLeftMark(ps.left_mark.saturating_add(position))
    });
    // TODO: Add keybindings for left right scrolling

    map.map_resize(|ev, _| {
        let Event::Resize(cols, rows) = ev else {
            unreachable!();
        };
        InputEvent::UpdateTermArea(cols as usize, rows as usize)
    });

    map.map_wild_event(|ev, _| {
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

#[cfg(test)]
mod tests;
