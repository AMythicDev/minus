#![allow(clippy::shadow_unrelated)]
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use super::*;

use crate::{
    input::{DefaultInputHandler, InputHandler},
    LineNumbers, Pager,
};
use std::fmt::Write;

#[test]
fn short_no_line_numbers() {
    let lines = "A line\nAnother line";
    let mut pager = Pager::new().set_text(lines);

    let mut out = Vec::with_capacity(lines.len());
    let rows = 10;

    assert!(write_lines(&mut out, &mut pager, rows,).is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark += 1;
    let rows = 10;

    assert!(write_lines(&mut out, &mut pager, rows,).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert_eq!(
        "\rA line\n\rAnother line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);
}

#[test]
fn long_no_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new().set_text(lines);
    let rows = 3;

    assert!(write_lines(&mut out, &mut pager, rows,).is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n\rThird line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.lines = "Another line\nThird line\nFourth line\nFifth line".to_string();
    pager.upper_mark = 1;
    let rows = 3;

    assert!(write_lines(&mut out, &mut pager, rows,).is_ok());

    assert_eq!(
        "\rThird line\n\rFourth line\n\rFifth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    let rows = 3;
    pager.upper_mark = 2;

    assert!(write_lines(&mut out, &mut pager, rows,).is_ok());

    assert_eq!(
        "\rThird line\n\rFourth line\n\rFifth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 1);
}

#[test]
fn short_with_line_numbers() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::Enabled);
    let rows = 10;

    assert!(write_lines(&mut out, &mut pager, rows).is_ok());

    assert_eq!(
        "\r1. A line\n\r2. Another line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;
    pager.line_numbers = LineNumbers::AlwaysOn;
    let rows = 10;

    assert!(write_lines(&mut out, &mut pager, rows,).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert_eq!(
        "\r1. A line\n\r2. Another line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);
}

#[test]
fn long_with_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::Enabled);
    let rows = 3;

    assert!(write_lines(&mut out, &mut pager, rows).is_ok());

    assert_eq!(
        "\r1. A line\n\r2. Another line\n\r3. Third line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;
    let rows = 3;

    assert!(write_lines(&mut out, &mut pager, rows).is_ok());

    assert_eq!(
        "\r2. Another line\n\r3. Third line\n\r4. Fourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 2;
    let rows = 3;

    assert!(write_lines(&mut out, &mut pager, rows).is_ok());

    assert_eq!(
        "\r2. Another line\n\r3. Third line\n\r4. Fourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 1);
}

#[test]
fn big_line_numbers_are_padded() {
    let lines = {
        let mut l = String::with_capacity(450);
        for i in 0..110 {
            writeln!(&mut l, "L{}", i).unwrap();
        }
        l
    };

    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::AlwaysOn);
    pager.upper_mark = 95;
    let rows = 10;

    assert!(write_lines(&mut out, &mut pager, rows).is_ok());

    // The padding should have inserted a space before the numbers that are less than 100.
    assert_eq!(
        "\r 96. L95\n\r 97. L96\n\r 98. L97\n\r 99. L98\n\r100. L99\n\r101. L100\n\r102. L101\n\r103. L102\n\r104. L103\n\r105. L104\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 95);
}

#[test]
fn line_numbers_not() {
    #[allow(clippy::enum_glob_use)]
    use LineNumbers::*;

    assert_eq!(AlwaysOn, !AlwaysOn);
    assert_eq!(AlwaysOff, !AlwaysOff);
    assert_eq!(Enabled, !Disabled);
    assert_eq!(Disabled, !Enabled);
}

#[test]
fn line_numbers_invertible() {
    #[allow(clippy::enum_glob_use)]
    use LineNumbers::*;

    assert!(!AlwaysOn.is_invertible());
    assert!(!AlwaysOff.is_invertible());
    assert!(Enabled.is_invertible());
    assert!(Disabled.is_invertible());
}

#[test]
fn draw_short_no_line_numbers() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::AlwaysOff);
    let rows = 10;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line\n"));
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;
    let rows = 10;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line\n"));
    assert_eq!(pager.upper_mark, 0);
}

#[test]
fn draw_long_no_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new().set_text(lines);
    let rows = 3;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line\n"));
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;
    let rows = 3;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rAnother line\n\rThird line\n"));
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 3;
    let rows = 3;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rThird line\n\rFourth line\n"));
    assert_eq!(pager.upper_mark, 2);
}

#[test]
fn draw_short_with_line_numbers() {
    let lines = "A line\nAnother line";
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::Enabled);
    let rows = 10;

    assert!(draw(&mut out, &mut pager, rows).is_ok());
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r1. A line\n\r2. Another line\n"));
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;
    let rows = 10;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r1. A line\n\r2. Another line\n"));
    assert_eq!(pager.upper_mark, 0);
}

#[test]
fn draw_long_with_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::Enabled);
    let rows = 3;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r1. A line\n\r2. Another line\n"));
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;
    let rows = 3;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r2. Another line\n\r3. Third line\n"));
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 3;
    let rows = 3;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r3. Third line\n\r4. Fourth line\n"));
    assert_eq!(pager.upper_mark, 2);
}

#[test]
fn draw_big_line_numbers_are_padded() {
    let lines = {
        let mut l = String::with_capacity(450);
        for i in 0..110 {
            writeln!(&mut l, "L{}", i).unwrap();
        }
        l
    };

    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::Enabled);
    pager.upper_mark = 95;
    let rows = 10;

    assert!(draw(&mut out, &mut pager, rows).is_ok());

    // The padding should have inserted a space before the numbers that are less than 100.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains(
            "\r 96. L95\n\r 97. L96\n\r 98. L97\n\r 99. L98\n\r100. L99\n\r101. L100\n\r102. L101\n\r103. L102\n\r104. L103\n",
        )
    );
    assert_eq!(pager.upper_mark, 95);
}

#[test]
fn draw_help_message() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut pager = Pager::new()
        .set_text(lines)
        .set_line_numbers(LineNumbers::AlwaysOff);
    let rows = 10;

    draw(&mut out, &mut pager, rows).expect("Should have written");

    let res = String::from_utf8(out).expect("Should have written valid UTF-8");
    assert!(res.contains("minus"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn input_handling() {
    let upper_mark = 12;
    let ln = LineNumbers::Enabled;
    let rows = 5;

    let input_handler: Box<dyn InputHandler> = Box::new(DefaultInputHandler {});

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(upper_mark + 1)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(upper_mark - 1)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            input_handler.handle_input(
                ev,
                usize::MAX,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MIN)),
            input_handler.handle_input(
                ev,
                usize::MIN,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(upper_mark + 5)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(upper_mark - 5)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(0)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
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
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::SHIFT,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('G'),
            modifiers: KeyModifiers::SHIFT,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
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
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Resize(42, 35);
        assert_eq!(
            Some(InputEvent::UpdateRows(35)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
        });
        assert_eq!(
            Some(InputEvent::UpdateLineNumber(!ln)),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::Exit),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        });
        assert_eq!(
            Some(InputEvent::Exit),
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            None,
            input_handler.handle_input(
                ev,
                upper_mark,
                #[cfg(feature = "search")]
                SearchMode::Unknown,
                ln,
                rows
            )
        );
    }
}
#[cfg(feature = "async_std_lib")]
#[cfg(test)]
mod async_std_tests {
    use crate::Pager;
    use std::sync::atomic::Ordering;
    use std::sync::{atomic::AtomicBool, Arc};
    #[test]
    pub fn test_exit_callback() {
        let mut pager = Pager::new();
        let exited = Arc::new(AtomicBool::new(false));
        let exited_within_callback = exited.clone();
        pager.add_exit_callback(move || exited_within_callback.store(true, Ordering::Relaxed));
        pager.exit();

        assert_eq!(true, exited.load(Ordering::Relaxed));
    }
}
