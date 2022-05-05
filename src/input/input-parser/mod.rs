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
    map
});

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

    let (modifier_half, code_half) = text.split_at(text.rfind('-').unwrap_or(0));
    let mut code_half = code_half.to_string();
    if code_half.chars().nth(0) == Some('-') {
        code_half.remove(0);
    }
    let keymodifiers = parse_key_modifiers(&modifier_half);
    let keycode = parse_code(&code_half);

    KeyEvent {
        modifiers: keymodifiers,
        code: keycode,
    }
}

pub fn parse_code(text: &str) -> KeyCode {
    if text.len() == 1 {
        KeyCode::Char(text.chars().nth(0).unwrap())
    } else {
        SPECIAL_KEYS
            .get(text)
            .unwrap_or_else(|| panic!("Invalid special key '{}' given", text))
            .to_owned()
    }
}

fn parse_key_modifiers(c: &str) -> KeyModifiers {
    const MODIFIERS: [char; 3] = ['m', 'c', 's'];

    let mut keymodifiers = KeyModifiers::empty();

    let chars = c.chars();

    let mut chars_peek = chars.peekable();

    while let Some(m) = chars_peek.peek() {
        if !MODIFIERS.contains(m) && *m != '-' {
            break;
        } else if *m == '-' {
            chars_peek.next();
        } else if MODIFIERS.contains(m) {
            match m {
                'm' => keymodifiers.insert(KeyModifiers::ALT),
                'c' => keymodifiers.insert(KeyModifiers::CONTROL),
                's' => keymodifiers.insert(KeyModifiers::SHIFT),
                _ => {}
            }
            chars_peek.next();
        }
    }
    keymodifiers
}

#[cfg(test)]
mod key_modifier_tests {
    use crossterm::event::KeyModifiers;

    use crate::input::input_parser::parse_key_modifiers;

    #[test]
    fn test_none() {
        assert_eq!(parse_key_modifiers("a"), KeyModifiers::empty());
    }

    #[test]
    fn test_control() {
        assert_eq!(parse_key_modifiers("c-c"), KeyModifiers::CONTROL);
    }

    #[test]
    fn test_control_alt() {
        assert_eq!(
            parse_key_modifiers("c-m-c"),
            KeyModifiers::from_bits(KeyModifiers::CONTROL.bits() | KeyModifiers::ALT.bits())
                .unwrap()
        );
    }

    #[test]
    fn test_all() {
        assert_eq!(parse_key_modifiers("c-m-s-a"), KeyModifiers::all());
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
