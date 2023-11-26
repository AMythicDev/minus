#![allow(clippy::uninlined_format_args)]

pub mod keydefs;
pub mod mousedefs;

use crossterm::event::KeyModifiers;
use once_cell::sync::Lazy;
use std::collections::HashMap;

fn parse_tokens(mut text: &str) -> Vec<Token> {
    assert!(
        text.chars().all(|c| c.is_ascii()),
        "'{}': Non ascii sequence found in input sequence",
        text
    );
    text = text.trim();
    assert!(
        text.chars().any(|c| !c.is_whitespace()),
        "'{}': Whitespace character found in input sequence",
        text
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

    token_list
}

pub static MODIFIERS: Lazy<HashMap<char, KeyModifiers>> = Lazy::new(|| {
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
