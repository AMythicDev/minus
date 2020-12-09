use super::*;

use std::fmt::Write;

#[test]
fn short_no_line_numbers() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 10;

    assert!(write_lines(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOff
    )
    .is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 10;

    assert!(write_lines(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::Disabled
    )
    .is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert_eq!(
        "\rA line\n\rAnother line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);
}

#[test]
fn long_no_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 3;

    assert!(write_lines(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::Disabled
    )
    .is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n\rThird line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 3;

    assert!(write_lines(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOff
    )
    .is_ok());

    assert_eq!(
        "\rAnother line\n\rThird line\n\rFourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 2;
    let rows = 3;

    assert!(write_lines(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::Disabled
    )
    .is_ok());

    assert_eq!(
        "\rAnother line\n\rThird line\n\rFourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 1);
}

#[test]
fn short_with_line_numbers() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 10;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Enabled).is_ok());

    assert_eq!(
        "\r1. A line\n\r2. Another line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 10;

    assert!(write_lines(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOn
    )
    .is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert_eq!(
        "\r1. A line\n\r2. Another line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);
}

#[test]
fn long_with_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 3;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Enabled).is_ok());

    assert_eq!(
        "\r1. A line\n\r2. Another line\n\r3. Third line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 3;

    assert!(write_lines(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOn
    )
    .is_ok());

    assert_eq!(
        "\r2. Another line\n\r3. Third line\n\r4. Fourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 2;
    let rows = 3;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Enabled).is_ok());

    assert_eq!(
        "\r2. Another line\n\r3. Third line\n\r4. Fourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 1);
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
    let mut upper_mark = 95;
    let rows = 10;

    assert!(write_lines(
        &mut out,
        &lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOn
    )
    .is_ok());

    // The padding should have inserted a space before the numbers that are less than 100.
    assert_eq!(
        "\r 96. L95\n\r 97. L96\n\r 98. L97\n\r 99. L98\n\r100. L99\n\r101. L100\n\r102. L101\n\r103. L102\n\r104. L103\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 95);
}

#[test]
fn line_numbers_not() {
    use LineNumbers::*;

    assert_eq!(AlwaysOn, !AlwaysOn);
    assert_eq!(AlwaysOff, !AlwaysOff);
    assert_eq!(Enabled, !Disabled);
    assert_eq!(Disabled, !Enabled);
}

#[test]
fn line_numbers_invertible() {
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
    let mut upper_mark = 0;
    let rows = 10;

    assert!(draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOff
    )
    .is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line\n"));
    assert_eq!(upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 10;

    assert!(draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::Disabled
    )
    .is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line\n"));
    assert_eq!(upper_mark, 0);
}

#[test]
fn draw_long_no_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 3;

    assert!(draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::Disabled
    )
    .is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line\n\rThird line\n"));
    assert_eq!(upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 3;

    assert!(draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOff
    )
    .is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rAnother line\n\rThird line\n\rFourth line\n"));
    assert_eq!(upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 2;
    let rows = 3;

    assert!(draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::Disabled
    )
    .is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rAnother line\n\rThird line\n\rFourth line\n"));
    assert_eq!(upper_mark, 1);
}

#[test]
fn draw_short_with_line_numbers() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 10;

    assert!(draw(&mut out, lines, rows, &mut upper_mark, LineNumbers::Enabled).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r1. A line\n\r2. Another line\n"));
    assert_eq!(upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 10;

    assert!(draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOn
    )
    .is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r1. A line\n\r2. Another line\n"));
    assert_eq!(upper_mark, 0);
}

#[test]
fn draw_long_with_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 3;

    assert!(draw(&mut out, lines, rows, &mut upper_mark, LineNumbers::Enabled).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r1. A line\n\r2. Another line\n\r3. Third line\n"));
    assert_eq!(upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 3;

    assert!(draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOn
    )
    .is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r2. Another line\n\r3. Third line\n\r4. Fourth line\n"));
    assert_eq!(upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 2;
    let rows = 3;

    assert!(draw(&mut out, lines, rows, &mut upper_mark, LineNumbers::Enabled).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r2. Another line\n\r3. Third line\n\r4. Fourth line\n"));
    assert_eq!(upper_mark, 1);
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
    let mut upper_mark = 95;
    let rows = 10;

    assert!(draw(
        &mut out,
        &lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOn
    )
    .is_ok());

    // The padding should have inserted a space before the numbers that are less than 100.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains(
            "\r 96. L95\n\r 97. L96\n\r 98. L97\n\r 99. L98\n\r100. L99\n\r101. L100\n\r102. L101\n\r103. L102\n\r104. L103\n",
        )
    );
    assert_eq!(upper_mark, 95);
}

#[test]
fn draw_help_message() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 10;

    draw(
        &mut out,
        lines,
        rows,
        &mut upper_mark,
        LineNumbers::AlwaysOff,
    )
    .expect("Should have written");

    let res = String::from_utf8(out).expect("Should have written valid UTF-8");
    assert!(res.contains("Press q or Ctrl+C to quit"));
    assert!(!res.contains(", Ctrl+L to display/hide line numbers"));

    let mut out = Vec::with_capacity(lines.len());

    assert!(draw(&mut out, lines, rows, &mut upper_mark, LineNumbers::Enabled,).is_ok());

    let res = String::from_utf8(out).expect("Should have written valid UTF-8");
    assert!(res.contains("Press q or Ctrl+C to quit"));
    assert!(res.contains(", Ctrl+L to display/hide line numbers"));
}

#[test]
fn input_handling() {
    let upper_mark = 12;
    let ln = LineNumbers::Enabled;

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(upper_mark + 1)),
            handle_input(ev, upper_mark, ln)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(upper_mark - 1)),
            handle_input(ev, upper_mark, ln)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Down,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            handle_input(ev, usize::MAX, ln)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Up,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MIN)),
            handle_input(ev, usize::MIN, ln)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MIN)),
            handle_input(ev, upper_mark, ln)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('g'),
            modifiers: KeyModifiers::SHIFT,
        });
        assert_eq!(
            Some(InputEvent::UpdateUpperMark(usize::MAX)),
            handle_input(ev, upper_mark, ln)
        );
    }

    {
        let ev = Event::Resize(42, 35);
        assert_eq!(
            Some(InputEvent::UpdateRows(35)),
            handle_input(ev, upper_mark, ln)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('l'),
            modifiers: KeyModifiers::CONTROL,
        });
        assert_eq!(
            Some(InputEvent::UpdateLineNumber(!ln)),
            handle_input(ev, upper_mark, ln)
        );
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(Some(InputEvent::Exit), handle_input(ev, upper_mark, ln));
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
        });
        assert_eq!(Some(InputEvent::Exit), handle_input(ev, upper_mark, ln));
    }

    {
        let ev = Event::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(None, handle_input(ev, upper_mark, ln));
    }
}
