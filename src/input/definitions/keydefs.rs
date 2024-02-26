#![allow(dead_code)]

use super::{Token, MODIFIERS};
use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyEventState, KeyModifiers};
use once_cell::sync::Lazy;

static SPECIAL_KEYS: Lazy<HashMap<&str, KeyCode>> = Lazy::new(|| {
    let mut map = HashMap::new();

    map.insert("enter", KeyCode::Enter);
    map.insert("tab", KeyCode::Tab);
    map.insert("backtab", KeyCode::BackTab);
    map.insert("backspace", KeyCode::Backspace);
    map.insert("up", KeyCode::Up);
    map.insert("down", KeyCode::Down);
    map.insert("right", KeyCode::Right);
    map.insert("left", KeyCode::Left);
    map.insert("pageup", KeyCode::PageUp);
    map.insert("pagedown", KeyCode::PageDown);
    map.insert("home", KeyCode::Home);
    map.insert("end", KeyCode::End);
    map.insert("insert", KeyCode::Insert);
    map.insert("delete", KeyCode::Delete);
    map.insert("esc", KeyCode::Esc);
    map.insert("f1", KeyCode::F(1));
    map.insert("f2", KeyCode::F(2));
    map.insert("f3", KeyCode::F(3));
    map.insert("f4", KeyCode::F(4));
    map.insert("f5", KeyCode::F(5));
    map.insert("f6", KeyCode::F(6));
    map.insert("f7", KeyCode::F(7));
    map.insert("f8", KeyCode::F(8));
    map.insert("f9", KeyCode::F(9));
    map.insert("f10", KeyCode::F(10));
    map.insert("f11", KeyCode::F(11));
    map.insert("f12", KeyCode::F(12));
    map.insert("dash", KeyCode::Char('-'));
    map.insert("space", KeyCode::Char(' '));

    map
});

struct KeySeq {
    code: Option<KeyCode>,
    modifiers: KeyModifiers,
}

impl Default for KeySeq {
    fn default() -> Self {
        Self {
            code: None,
            modifiers: KeyModifiers::NONE,
        }
    }
}

pub fn parse_key_event(text: &str) -> KeyEvent {
    let token_list = super::parse_tokens(text);

    KeySeq::gen_keyevent_from_tokenlist(&token_list, text)
}

impl KeySeq {
    fn gen_keyevent_from_tokenlist(token_list: &[Token], text: &str) -> KeyEvent {
        let mut ks = Self::default();

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
                    if let Some(m) = MODIFIERS.get(c) {
                        if token_iter.next() == Some(&Token::Separator) {
                            assert!(
                                !ks.modifiers.contains(*m),
                                "'{}': Multiple instances of same modifier given",
                                text
                            );
                            ks.modifiers.insert(*m);
                        } else if ks.code.is_none() {
                            ks.code = Some(KeyCode::Char(*c));
                        } else {
                            panic!("'{}' Invalid key input sequence given", text);
                        }
                    } else if ks.code.is_none() {
                        ks.code = Some(KeyCode::Char(*c));
                    } else {
                        panic!("'{}': Invalid key input sequence given", text);
                    }
                }
                Token::MultipleChar(c) => {
                    let c = c.to_ascii_lowercase().to_string();
                    SPECIAL_KEYS.get(c.as_str()).map_or_else(
                        || panic!("'{}': Invalid key input sequence given", text),
                        |key| {
                            if ks.code.is_none() {
                                ks.code = Some(*key);
                            } else {
                                panic!("'{}': Invalid key input sequence given", text);
                            }
                        },
                    );
                    token_iter.next();
                }
            }
        }
        KeyEvent {
            code: ks.code.unwrap_or(KeyCode::Null),
            modifiers: ks.modifiers,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }
}

#[cfg(test)]
#[test]
#[allow(clippy::too_many_lines)]
fn test_parse_key_event() {
    assert_eq!(
        parse_key_event("up"),
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("k"),
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("j"),
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("down"),
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("down"),
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("enter"),
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("c-u"),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("c-d"),
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("g"),
        KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("s-g"),
        KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::SHIFT,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("G"),
        KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("pageup"),
        KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("pagedown"),
        KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("c-l"),
        KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("q"),
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("c-c"),
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("/"),
        KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("?"),
        KeyEvent {
            code: KeyCode::Char('?'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("n"),
        KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("p"),
        KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
    assert_eq!(
        parse_key_event("c-s-h"),
        KeyEvent {
            code: KeyCode::Char('h'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    );
}
