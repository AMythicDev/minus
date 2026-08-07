pub mod keydefs;
pub mod mousedefs;

use crossterm::event::KeyModifiers;
use std::{collections::HashMap, sync::LazyLock};

fn parse_tokens(mut text: &str) -> Vec<Token> {
    assert!(
        text.is_ascii(),
        "'{text}': Non ascii sequence found in input sequence",
    );
    text = text.trim();
    assert!(
        text.chars().any(|c| !c.is_whitespace()),
        "'{text}': Whitespace character found in input sequence",
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

pub static MODIFIERS: LazyLock<HashMap<char, KeyModifiers>> = LazyLock::new(|| {
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
