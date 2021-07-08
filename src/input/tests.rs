use crate::{input::InputEvent, LineNumbers, Pager, SearchMode};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

// Just a transparent function to fix incompatiblity issues between
// versions
// TODO: Remove this later in favour of how handle_event should actually be called
fn handle_input(ev: Event, p: &Pager) -> Option<InputEvent> {
    p.input_handler
        .handle_input(ev, p.upper_mark, p.search_mode, p.line_numbers, p.rows)
}

// Keyboard navigation
#[test]
#[allow(clippy::too_many_lines)]
fn test_kb_nav() {
    let mut pager = Pager::new().unwrap();
    pager.upper_mark = 12;
    pager.set_line_numbers(LineNumbers::Enabled);
    pager.rows = 5;

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(pager.upper_mark + 1)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(pager.upper_mark - 1)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(0)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::PageUp,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            // rows is 5, therefore upper_mark = upper_mark - rows -1
            Some(InputEvent::UpdateUpperMark(8)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::SHIFT,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::SHIFT,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::PageDown,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            // rows is 5, therefore upper_mark = upper_mark - rows -1
            Some(InputEvent::UpdateUpperMark(16)),
            handle_input(ev, &pager)
        );
    }

    {
        // Half page down
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('d'),
            modifiers: KeyModifiers::CONTROL,
        });
        // Rows is 5 and upper_mark is at 12 so result should be 14
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(14)),
            handle_input(ev, &pager)
        );
    }

    {
        // Half page up
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('u'),
            modifiers: KeyModifiers::CONTROL,
        });
        // Rows is 5 and upper_mark is at 12 so result should be 10
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(10)),
            handle_input(ev, &pager)
        );
    }
}

#[test]
fn test_mouse_nav() {
    let mut pager = Pager::new().unwrap();
    pager.upper_mark = 12;
    pager.set_line_numbers(LineNumbers::Enabled);
    pager.rows = 5;
    {
        let ev = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            row: 0,
            column: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(
            Some(InputEvent::UpdateUpperMark(pager.upper_mark + 5)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            row: 0,
            column: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(pager.upper_mark - 5)),
            handle_input(ev, &pager)
        );
    }
}

#[test]
fn test_saturation() {
    let mut pager = Pager::new().unwrap();
    pager.upper_mark = 12;
    pager.set_line_numbers(LineNumbers::Enabled);
    pager.rows = 5;

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        });
        // Pager for local use
        let mut pager = Pager::new().unwrap();
        pager.upper_mark = usize::MAX;
        pager.set_line_numbers(LineNumbers::Enabled);
        pager.rows = 5;
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        });
        // Pager for local use
        let mut pager = Pager::new().unwrap();
        pager.upper_mark = usize::MIN;
        pager.set_line_numbers(LineNumbers::Enabled);
        pager.rows = 5;
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MIN)),
            handle_input(ev, &pager)
        );
    }
}

#[test]
fn test_misc_events() {
    let mut pager = Pager::new().unwrap();
    pager.upper_mark = 12;
    pager.set_line_numbers(LineNumbers::Enabled);
    pager.rows = 5;

    {
        let ev = Event::Resize(42, 35);
        assert_eq!(
            Some(InputEvent::UpdateTermArea(42, 35)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
        });
        assert_eq!(
            Some(InputEvent::UpdateLineNumber(!pager.line_numbers)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(Some(InputEvent::Exit), handle_input(ev, &pager));
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        });
        assert_eq!(Some(InputEvent::Exit), handle_input(ev, &pager));
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(None, handle_input(ev, &pager));
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn test_search_bindings() {
    let mut pager = Pager::new().unwrap();
    pager.upper_mark = 12;
    pager.set_line_numbers(LineNumbers::Enabled);
    pager.rows = 5;

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('/'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::Search(SearchMode::Forward)),
            handle_input(ev, &pager)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('?'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::Search(SearchMode::Reverse)),
            handle_input(ev, &pager)
        );
    }
    {
        // NextMatch and PrevMatch forward search
        let next_event = Event::Key(KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::NONE,
        });
        let prev_event = Event::Key(KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(
            pager.input_handler.handle_input(
                next_event,
                pager.upper_mark,
                SearchMode::Forward,
                pager.line_numbers,
                pager.rows
            ),
            Some(InputEvent::NextMatch)
        );
        assert_eq!(
            pager.input_handler.handle_input(
                prev_event,
                pager.upper_mark,
                SearchMode::Forward,
                pager.line_numbers,
                pager.rows
            ),
            Some(InputEvent::PrevMatch)
        )
    }

    {
        // NextMatch and PrevMatch reverse search
        let next_event = Event::Key(KeyEvent {
            code: KeyCode::Char('n'),
            modifiers: KeyModifiers::NONE,
        });
        let prev_event = Event::Key(KeyEvent {
            code: KeyCode::Char('p'),
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(
            pager.input_handler.handle_input(
                next_event,
                pager.upper_mark,
                SearchMode::Reverse,
                pager.line_numbers,
                pager.rows
            ),
            Some(InputEvent::PrevMatch)
        );
        assert_eq!(
            pager.input_handler.handle_input(
                prev_event,
                pager.upper_mark,
                SearchMode::Reverse,
                pager.line_numbers,
                pager.rows
            ),
            Some(InputEvent::NextMatch)
        )
    }
}
