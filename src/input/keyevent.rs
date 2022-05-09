#![allow(dead_code)]

use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    map.insert("Delete", KeyCode::Delete);
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

    map
});

static MODIFIERS: Lazy<HashMap<char, KeyModifiers>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert('m', KeyModifiers::ALT);
    map.insert('c', KeyModifiers::CONTROL);
    map.insert('s', KeyModifiers::SHIFT);

    map
});

#[derive(Debug, PartialEq)]
enum Token {
    Separator, // -
    SingleChar(char),
    MultipleChar(String),
}

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

pub fn parse_key_event(mut text: &str) -> KeyEvent {
    assert!(
        text.chars().all(|c| c.is_ascii()),
        "Non ascii sequence found in input sequence"
    );
    text = text.trim();
    assert!(
        text.chars().any(|c| !c.is_whitespace()),
        "Whitespace character found in input sequence"
    );

    let mut token_list = Vec::with_capacity(text.len());

    let mut chars_peek = text.chars().peekable();

    let mut s = String::with_capacity(5);

    let flush_s = |s: &mut String, token_list: &mut Vec<Token>| {
        match s.len() {
            1 => token_list.push(Token::SingleChar(s.chars().next().unwrap())),
            2.. => token_list.push(Token::MultipleChar(s.clone())),
            _ => {}
        }
        s.clear();
    };

    while let Some(chr) = chars_peek.peek() {
        match chr {
            '-' => {
                flush_s(&mut s, &mut token_list);
                token_list.push(Token::Separator);
            }
            c => {
                s.push(*c);
            }
        }
        chars_peek.next();
    }
    flush_s(&mut s, &mut token_list);

    KeySeq::gen_keyevent_from_tokenlist(&token_list)
}

impl KeySeq {
    fn gen_keyevent_from_tokenlist(token_list: &[Token]) -> KeyEvent {
        let mut ks = Self::default();

        let mut token_iter = token_list.iter().peekable();

        while let Some(token) = token_iter.peek() {
            match token {
                Token::Separator => {
                    token_iter.next();
                    if token_iter.peek() == Some(&&Token::Separator) {
                        panic!("Multiple - separators found consecutively");
                    }
                }
                Token::SingleChar(c) => {
                    token_iter.next();
                    if let Some(m) = MODIFIERS.get(c) {
                        if token_iter.next() == Some(&Token::Separator) {
                            assert!(
                                !ks.modifiers.contains(*m),
                                "Multiple instances of same modifier given"
                            );
                            ks.modifiers.insert(*m);
                        } else if ks.code.is_none() {
                            ks.code = Some(KeyCode::Char(*c));
                        } else {
                            panic!("Invalid key input sequence given");
                        }
                    } else if ks.code.is_none() {
                        ks.code = Some(KeyCode::Char(*c));
                    } else {
                        panic!("Invalid key input sequence given");
                    }
                }
                Token::MultipleChar(c) => {
                    let c = c.to_ascii_lowercase().to_string();
                    SPECIAL_KEYS.get(c.as_str()).map_or_else(
                        || panic!("Invalid key input sequence given"),
                        |key| {
                            if ks.code.is_none() {
                                ks.code = Some(*key);
                            } else {
                                panic!("Invalid key input sequence given");
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
        }
    }
}

#[cfg(test)]
#[test]
fn test_parse_key_event() {
    assert_eq!(
        parse_key_event("up"),
        KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("k"),
        KeyEvent {
            code: KeyCode::Char('k'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("j"),
        KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("down"),
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("down"),
        KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("enter"),
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("c-u"),
        KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL
        }
    );
    assert_eq!(
        parse_key_event("c-d"),
        KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL
        }
    );
    assert_eq!(
        parse_key_event("g"),
        KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("s-g"),
        KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::SHIFT
        }
    );
    assert_eq!(
        parse_key_event("G"),
        KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("pageup"),
        KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("pagedown"),
        KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("c-l"),
        KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL
        }
    );
    assert_eq!(
        parse_key_event("q"),
        KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("c-c"),
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL
        }
    );
    assert_eq!(
        parse_key_event("/"),
        KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("?"),
        KeyEvent {
            code: KeyCode::Char('?'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("n"),
        KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::NONE
        }
    );
    assert_eq!(
        parse_key_event("p"),
        KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE
        }
    );
}
