//! Help text and related definitions for the pager.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fmt::Write as _;

/// Format a [`KeyEvent`] into a human-readable representation (e.g. `"Ctrl-c"`, `"Alt-h"`).
#[must_use]
pub fn format_key(ke: &KeyEvent) -> String {
    let mut s = String::new();
    if ke.modifiers.contains(KeyModifiers::CONTROL) {
        s.push_str("Ctrl-");
    }
    if ke.modifiers.contains(KeyModifiers::ALT) {
        s.push_str("Alt-");
    }
    if ke.modifiers.contains(KeyModifiers::SHIFT) {
        if let KeyCode::Char(c) = ke.code {
            if !c.is_ascii_uppercase() {
                s.push_str("Shift-");
            }
        } else {
            s.push_str("Shift-");
        }
    }

    match ke.code {
        KeyCode::Char(c) => s.push(c),
        KeyCode::Enter => s.push_str("Enter"),
        KeyCode::Tab => s.push_str("Tab"),
        KeyCode::BackTab => s.push_str("BackTab"),
        KeyCode::Backspace => s.push_str("Backspace"),
        KeyCode::Esc => s.push_str("Esc"),
        KeyCode::Up => s.push_str("Up"),
        KeyCode::Down => s.push_str("Down"),
        KeyCode::Left => s.push_str("Left"),
        KeyCode::Right => s.push_str("Right"),
        KeyCode::PageUp => s.push_str("PageUp"),
        KeyCode::PageDown => s.push_str("PageDown"),
        KeyCode::Home => s.push_str("Home"),
        KeyCode::End => s.push_str("End"),
        KeyCode::Delete => s.push_str("Delete"),
        KeyCode::Insert => s.push_str("Insert"),
        KeyCode::F(n) => {
            let _ = write!(s, "F{n}");
        }
        KeyCode::Null => s.push_str("Null"),
        _ => s.push_str("Unknown"),
    }
    s
}

/// Format dynamic help table from key event entries and their descriptions.
///
/// Empty descriptions are omitted.
#[must_use]
pub fn format_help_table_from_entries<'a, I>(entries: I) -> String
where
    I: IntoIterator<Item = (&'a KeyEvent, &'a str)>,
{
    let mut groups: Vec<(&'a str, Vec<String>)> = Vec::new();
    for (key, desc) in entries {
        let trimmed_desc = desc.trim();
        if trimmed_desc.is_empty() {
            continue;
        }
        let key_str = format_key(key);
        if let Some((_, keys)) = groups.iter_mut().find(|(d, _)| *d == trimmed_desc) {
            if !keys.contains(&key_str) {
                keys.push(key_str);
            }
        } else {
            groups.push((trimmed_desc, vec![key_str]));
        }
    }

    if groups.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("                         COMMAND SUMMARY\n\n");
    out.push_str("  Key(s)                         Action\n");
    out.push_str("  ------                         ------\n");

    for (desc, keys) in groups {
        let keys_str = keys.join(", ");
        let _ = writeln!(out, "  {keys_str:<30} {desc}");
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    #[test]
    fn test_format_key() {
        let k1 = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        assert_eq!(format_key(&k1), "q");

        let k2 = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        assert_eq!(format_key(&k2), "Ctrl-c");

        let k3 = KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::ALT,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        assert_eq!(format_key(&k3), "Alt-Up");
    }

    #[test]
    fn test_format_help_table_from_entries() {
        let k1 = KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let k2 = KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        };
        let k3 = KeyEvent {
            code: KeyCode::Char('x'),
            modifiers: KeyModifiers::NONE,
            kind: crossterm::event::KeyEventKind::Press,
            state: KeyEventState::NONE,
        };

        let entries = vec![(&k1, "quit"), (&k2, "quit"), (&k3, "")];
        let table = format_help_table_from_entries(entries);
        assert!(table.contains("COMMAND SUMMARY"));
        assert!(table.contains("q, Ctrl-c"));
        assert!(table.contains("quit"));
        assert!(!table.contains(" x "));

        let empty_table = format_help_table_from_entries(Vec::<(&KeyEvent, &str)>::new());
        assert!(empty_table.is_empty());
    }
}
