#![allow(clippy::shadow_unrelated)]
use super::*;

use crate::{LineNumbers, PagerState};
use std::fmt::Write;

// * In some places, where test lines are close to the row, 1 should be added
// to the rows because `write_lines` does care about the prompt

// The pager assumes 80 columns and 10 rows in tests
// Wherever the tests require this 80x10 configuration, no explicit assignment is done
// In other cases, the tests do set the their required values

#[test]
fn short_no_line_numbers() {
    let lines = "A line\nAnother line";
    let mut pager = PagerState::new().unwrap();

    pager.lines = lines.to_string();
    pager.format_lines();

    let mut out = Vec::with_capacity(lines.len());

    assert!(write_lines(&mut out, &mut pager).is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark += 1;

    assert!(write_lines(&mut out, &mut pager).is_ok());

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
    let mut pager = PagerState::new().unwrap();
    // One extra line for prompt
    pager.rows = 4;
    pager.lines = lines.to_string();
    pager.format_lines();

    assert!(write_lines(&mut out, &mut pager).is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n\rThird line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.lines = "Another line\nThird line\nFourth line\nFifth line\n".to_string();
    pager.upper_mark = 1;
    pager.format_lines();

    assert!(write_lines(&mut out, &mut pager).is_ok());

    assert_eq!(
        "\rThird line\n\rFourth line\n\rFifth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 2;

    assert!(write_lines(&mut out, &mut pager).is_ok());

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
    let mut pager = PagerState::new().unwrap();
    pager.lines = lines.to_string();
    pager.line_numbers = LineNumbers::Enabled;
    pager.format_lines();

    assert!(write_lines(&mut out, &mut pager).is_ok());

    assert_eq!(
        "\r     1. A line\n\r     2. Another line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;
    pager.line_numbers = LineNumbers::AlwaysOn;

    assert!(write_lines(&mut out, &mut pager).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert_eq!(
        "\r     1. A line\n\r     2. Another line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);
}

#[test]
fn long_with_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = PagerState::new().unwrap();
    pager.rows = 4;
    pager.lines = lines.to_string();
    pager.line_numbers = LineNumbers::Enabled;
    pager.format_lines();

    assert!(write_lines(&mut out, &mut pager).is_ok());

    assert_eq!(
        "\r     1. A line\n\r     2. Another line\n\r     3. Third line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;

    assert!(write_lines(&mut out, &mut pager).is_ok());

    assert_eq!(
        "\r     2. Another line\n\r     3. Third line\n\r     4. Fourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 2;

    assert!(write_lines(&mut out, &mut pager).is_ok());

    assert_eq!(
        "\r     2. Another line\n\r     3. Third line\n\r     4. Fourth line\n",
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
    let mut pager = PagerState::new().unwrap();
    pager.upper_mark = 95;
    pager.rows = 11;
    pager.lines = lines;
    pager.line_numbers = LineNumbers::AlwaysOn;
    pager.format_lines();

    assert!(write_lines(&mut out, &mut pager).is_ok());

    // The padding should have inserted a space before the numbers that are less than 100.
    assert_eq!(
        "\r      96. L95\n\r      97. L96\n\r      98. L97\n\r      99. L98\n\r     100. \
         L99\n\r     101. L100\n\r     102. L101\n\r     103. L102\n\r     104. L103\n\r     105. L104\n",
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
    let mut pager = PagerState::new().unwrap();
    pager.lines = lines.to_string();
    pager.line_numbers = LineNumbers::AlwaysOff;
    pager.format_lines();

    assert!(draw_full(&mut out, &mut pager).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line"));
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;

    assert!(draw_full(&mut out, &mut pager).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line"));
    assert_eq!(pager.upper_mark, 0);
}

#[test]
fn draw_long_no_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = PagerState::new().unwrap();
    pager.rows = 3;
    pager.lines = lines.to_string();
    pager.format_lines();

    assert!(draw_full(&mut out, &mut pager).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rA line\n\rAnother line"));
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;

    assert!(draw_full(&mut out, &mut pager).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rAnother line\n\rThird line"));
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 3;

    assert!(draw_full(&mut out, &mut pager).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\rThird line\n\rFourth line"));
    assert_eq!(pager.upper_mark, 2);
}

#[test]
fn draw_short_with_line_numbers() {
    let lines = "A line\nAnother line";
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = PagerState::new().unwrap();
    pager.lines = lines.to_string();
    pager.line_numbers = LineNumbers::Enabled;
    pager.format_lines();

    assert!(draw_full(&mut out, &mut pager).is_ok());
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r     1. A line\n\r     2. Another line"));
    assert_eq!(pager.upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;

    assert!(draw_full(&mut out, &mut pager).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r     1. A line\n\r     2. Another line"));
    assert_eq!(pager.upper_mark, 0);
}

#[test]
fn draw_long_with_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    // Displaying as much of the lines as possible from the start.
    let mut out = Vec::with_capacity(lines.len());
    let mut pager = PagerState::new().unwrap();
    pager.rows = 3;
    pager.lines = lines.to_string();
    pager.line_numbers = LineNumbers::Enabled;
    pager.format_lines();

    assert!(draw_full(&mut out, &mut pager).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r     1. A line\n\r     2. Another line"));
    assert_eq!(pager.upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 1;

    assert!(draw_full(&mut out, &mut pager).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r     2. Another line\n\r     3. Third line"));
    assert_eq!(pager.upper_mark, 1);

    // This test ensures that as much text as possible will be displayed, even
    // when less is asked for.
    let mut out = Vec::with_capacity(lines.len());
    pager.upper_mark = 3;

    assert!(draw_full(&mut out, &mut pager).is_ok());

    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains("\r     3. Third line\n\r     4. Fourth line"));
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
    let mut pager = PagerState::new().unwrap();
    pager.upper_mark = 95;
    pager.lines = lines;
    pager.line_numbers = LineNumbers::Enabled;
    pager.format_lines();

    assert!(draw_full(&mut out, &mut pager).is_ok());

    // The padding should have inserted a space before the numbers that are less than 100.
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains(
            "\r      96. L95\n\r      97. L96\n\r      98. L97\n\r      99. L98\n\r     100. L99\n\r     101. L100\n\r     102. L101\n\r     103. L102\n\r     104. L103",
        )
    );
    assert_eq!(pager.upper_mark, 95);
}

#[test]
fn draw_wrapping_line_numbers() {
    let lines = (0..3)
        .map(|l| format!("Line {i}: This is the line who is {i}", i = l))
        .collect::<Vec<String>>()
        .join("\n");

    let mut out = Vec::new();
    let mut pager = PagerState::new().unwrap();
    pager.lines = lines;
    pager.cols = 30;
    pager.upper_mark = 2;
    pager.line_numbers = LineNumbers::Enabled;
    pager.format_lines();

    assert!(draw_full(&mut out, &mut pager).is_ok());

    let written = String::from_utf8(out).expect("Should have written valid UTF-8");
    let expected = "     2. Line 1: This is the\n\r        line who is 1\n\r     3. Line 2: This is the\n\r        line who is 2";
    assert!(written.contains(expected));
}

#[test]
fn draw_help_message() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut pager = PagerState::new().unwrap();
    pager.lines = lines.to_string();
    pager.line_numbers = LineNumbers::AlwaysOff;
    pager.format_prompt();

    draw_full(&mut out, &mut pager).expect("Should have written");

    let res = String::from_utf8(out).expect("Should have written valid UTF-8");
    assert!(res.contains("minus"));
}

#[test]
fn test_draw_no_overflow() {
    const TEXT: &str = "This is a line of text to the pager";
    let mut out = Vec::with_capacity(TEXT.len());
    let mut pager = PagerState::new().unwrap();
    pager.lines = TEXT.to_string();
    pager.format_lines();
    draw_full(&mut out, &mut pager).unwrap();
    assert!(String::from_utf8(out)
        .expect("Should have written valid UTF-8")
        .contains(TEXT));
}
