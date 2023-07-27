use std::collections::HashMap;

use super::{Token, MODIFIERS};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use once_cell::sync::Lazy;

static MOUSE_ACTIONS: Lazy<HashMap<&str, MouseEventKind>> = Lazy::new(|| {
    let mut map = HashMap::new();

    map.insert("left:down", MouseEventKind::Down(MouseButton::Left));
    map.insert("right:down", MouseEventKind::Down(MouseButton::Right));
    map.insert("mid:down", MouseEventKind::Down(MouseButton::Middle));

    map.insert("left:up", MouseEventKind::Up(MouseButton::Left));
    map.insert("right:up", MouseEventKind::Up(MouseButton::Right));
    map.insert("mid:up", MouseEventKind::Up(MouseButton::Middle));

    map.insert("left:drag", MouseEventKind::Drag(MouseButton::Left));
    map.insert("right:drag", MouseEventKind::Drag(MouseButton::Right));
    map.insert("mid:drag", MouseEventKind::Drag(MouseButton::Middle));

    map.insert("move", MouseEventKind::Moved);
    map.insert("scroll:up", MouseEventKind::ScrollUp);
    map.insert("scroll:down", MouseEventKind::ScrollDown);

    map
});

pub fn parse_mouse_event(text: &str) -> MouseEvent {
    let token_list = super::parse_tokens(text);
    gen_mouse_event_from_tokenlist(&token_list, text)
}

fn gen_mouse_event_from_tokenlist(token_list: &[Token], text: &str) -> MouseEvent {
    let mut kind = None;
    let mut modifiers = KeyModifiers::NONE;

    let mut token_iter = token_list.iter().peekable();

    while let Some(token) = token_iter.peek() {
        match token {
            Token::Separator => {
                token_iter.next();
                assert!(
                    !(token_iter.peek() == Some(&&Token::Separator)),
                    "'{}': Multiple separators found consecutively",
                    text
                );
            }
            Token::SingleChar(c) => {
                token_iter.next();
                MODIFIERS.get(c).map_or_else(
                    || {
                        panic!("'{}': Invalid keymodifier '{}' given", text, c);
                    },
                    |m| {
                        if token_iter.next() == Some(&Token::Separator) {
                            assert!(
                                !modifiers.contains(*m),
                                "'{}': Multiple instances of same modifier given",
                                text
                            );
                            modifiers.insert(*m);
                        } else {
                            panic!("'{}' Invalid key input sequence given", text);
                        }
                    },
                );
            }
            Token::MultipleChar(c) => {
                let c = c.to_ascii_lowercase().to_string();
                MOUSE_ACTIONS.get(c.as_str()).map_or_else(
                    || panic!("'{}': Invalid key input sequence given", text),
                    |k| {
                        if kind.is_none() {
                            kind = Some(*k);
                        } else {
                            panic!("'{}': Invalid key input sequence given", text);
                        }
                    },
                );
                token_iter.next();
            }
        }
    }
    MouseEvent {
        kind: kind.unwrap_or_else(|| panic!("No MouseEventKind found for '{}", text)),
        modifiers,
        row: 0,
        column: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_mouse_event;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

    #[test]
    fn test_without_modifiers() {
        assert_eq!(
            parse_mouse_event("left:down"),
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                modifiers: KeyModifiers::NONE,
                row: 0,
                column: 0,
            }
        );

        assert_eq!(
            parse_mouse_event("mid:up"),
            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Middle),
                modifiers: KeyModifiers::NONE,
                row: 0,
                column: 0,
            }
        );

        assert_eq!(
            parse_mouse_event("right:down"),
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Right),
                modifiers: KeyModifiers::NONE,
                row: 0,
                column: 0,
            }
        );
        assert_eq!(
            parse_mouse_event("scroll:up"),
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                modifiers: KeyModifiers::NONE,
                row: 0,
                column: 0,
            }
        );
        assert_eq!(
            parse_mouse_event("move"),
            MouseEvent {
                kind: MouseEventKind::Moved,
                modifiers: KeyModifiers::NONE,
                row: 0,
                column: 0,
            }
        );
    }

    #[test]
    fn test_with_modifiers() {
        assert_eq!(
            parse_mouse_event("m-left:down"),
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                modifiers: KeyModifiers::ALT,
                row: 0,
                column: 0,
            }
        );

        assert_eq!(
            parse_mouse_event("m-c-mid:up"),
            MouseEvent {
                kind: MouseEventKind::Up(MouseButton::Middle),
                modifiers: KeyModifiers::ALT | KeyModifiers::CONTROL,
                row: 0,
                column: 0,
            }
        );
        assert_eq!(
            parse_mouse_event("c-scroll:up"),
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                modifiers: KeyModifiers::CONTROL,
                row: 0,
                column: 0,
            }
        );
        assert_eq!(
            parse_mouse_event("m-c-s-move"),
            MouseEvent {
                kind: MouseEventKind::Moved,
                modifiers: KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL,
                row: 0,
                column: 0,
            }
        );
    }
}
