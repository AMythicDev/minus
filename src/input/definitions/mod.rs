pub mod keydefs;
<<<<<<< HEAD
<<<<<<< HEAD
pub mod mousedefs;
=======
>>>>>>> 3757de7 (input: Add definitions mod for better organization)
=======
pub mod mousedefs;
>>>>>>> e1c66ac (input: Fix clippy lints)

use crossterm::event::KeyModifiers;
use once_cell::sync::Lazy;
use std::collections::HashMap;

<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 056f2d9 (input/definitions: Refactor the code)
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

<<<<<<< HEAD
=======
>>>>>>> 3757de7 (input: Add definitions mod for better organization)
=======
>>>>>>> 056f2d9 (input/definitions: Refactor the code)
pub static MODIFIERS: Lazy<HashMap<char, KeyModifiers>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert('m', KeyModifiers::ALT);
    map.insert('c', KeyModifiers::CONTROL);
    map.insert('s', KeyModifiers::SHIFT);

    map
});
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 056f2d9 (input/definitions: Refactor the code)

#[derive(Debug, PartialEq)]
enum Token {
    Separator, // -
    SingleChar(char),
    MultipleChar(String),
}
<<<<<<< HEAD
=======
>>>>>>> 3757de7 (input: Add definitions mod for better organization)
=======
>>>>>>> 056f2d9 (input/definitions: Refactor the code)
