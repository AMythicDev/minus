use super::*;

#[test]
fn short_no_line_numbers() {
    let lines = "A line\nAnother line";

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 0;
    let rows = 10;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n",
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

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

    assert_eq!(
        "\rA line\n\rAnother line\n\rThird line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 3;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

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

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::No).is_ok());

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

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Yes).is_ok());

    assert_eq!(
        "\r1. A line\n\r2. Another line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 10;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Yes).is_ok());

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

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Yes).is_ok());

    assert_eq!(
        "\r1. A line\n\r2. Another line\n\r3. Third line\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 0);

    // This ensures that asking for a position other than 0 works.
    let mut out = Vec::with_capacity(lines.len());
    let mut upper_mark = 1;
    let rows = 3;

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Yes).is_ok());

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

    assert!(write_lines(&mut out, lines, rows, &mut upper_mark, LineNumbers::Yes).is_ok());

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

    assert!(write_lines(&mut out, &lines, rows, &mut upper_mark, LineNumbers::Yes).is_ok());

    // The padding should have inserted a space before the numbers that are less than 100.
    assert_eq!(
        "\r 96. L95\n\r 97. L96\n\r 98. L97\n\r 99. L98\n\r100. L99\n\r101. L100\n\r102. L101\n\r103. L102\n\r104. L103\n\r105. L104\n\r106. L105\n\r107. L106\n\r108. L107\n\r109. L108\n\r110. L109\n",
        String::from_utf8(out).expect("Should have written valid UTF-8")
    );
    assert_eq!(upper_mark, 95);
}
