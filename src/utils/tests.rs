use super::*;

#[test]
fn short_no_line_numbers() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 10;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

    assert_eq!(
        "A line\nAnother line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 10;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

    // The number of lines is less than 'rows' so 'upper_mark' will be 0 even
    // if we set it to 1. This is done because everything can be displayed without problems.
    assert_eq!(
        "A line\nAnother line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);
}

#[test]
fn long_no_line_numbers() {
    let lines = "A line\nAnother line\nThird line\nFourth line";

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 3;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

    assert_eq!(
        "A line\nAnother line\nThird line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 3;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

    assert_eq!(
        "Another line\nThird line\nFourth line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 1);
}
