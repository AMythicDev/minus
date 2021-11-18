use super::{rewrap, wrap_str, Pager};
use std::fmt::Write;

// Test the implementation of std::fmt::Write on Pager
#[test]
fn test_writeln() {
    const TEST: &str = "This is a line";
    let mut pager = Pager::new().unwrap();
    writeln!(pager, "{}", TEST).unwrap();
    assert_eq!(pager.lines, format!("{}\n", TEST));
}

#[test]
fn test_write() {
    const TEST: &str = "This is a line";
    let mut pager = Pager::new().unwrap();
    write!(pager, "{}", TEST).unwrap();
    assert_eq!(pager.formatted_lines, vec![TEST.to_string()]);
    assert_eq!(pager.lines, TEST.to_string());
}

#[test]
fn test_sequential_write() {
    const TEXT1: &str = "This is a line.";
    const TEXT2: &str = " This is a follow up line";
    let mut pager = Pager::new().unwrap();
    write!(pager, "{}", TEXT1).unwrap();
    write!(pager, "{}", TEXT2).unwrap();
    assert_eq!(pager.formatted_lines, vec![format!("{}{}", TEXT1, TEXT2)]);
    assert_eq!(pager.lines, TEXT1.to_string() + TEXT2);
}

#[test]
fn test_sequential_writeln() {
    const TEXT1: &str = "This is a line.";
    const TEXT2: &str = " This is a follow up line";
    let mut pager = Pager::new().unwrap();
    writeln!(pager, "{}", TEXT1).unwrap();
    writeln!(pager, "{}", TEXT2).unwrap();

    assert_eq!(
        pager.formatted_lines,
        vec![TEXT1.to_string(), TEXT2.to_string()]
    );
}

#[test]
fn test_crlf_write() {
    const LINES: [&str; 4] = [
        "hello,\n",
        "this is ",
        "a test\r\n",
        "of weird line endings",
    ];

    let mut pager = Pager::new().unwrap();

    for line in LINES {
        write!(pager, "{}", line).unwrap();
    }

    assert_eq!(
        pager.formatted_lines,
        vec![
            "hello,".to_string(),
            "this is a test".to_string(),
            "of weird line endings".to_string()
        ]
    );
}

#[test]
fn test_unusual_whitespace() {
    const LINES: [&str; 4] = [
        "This line has trailing whitespace      ",
        "     This has leading whitespace\n",
        "   This has whitespace on both sides   ",
        "Andthishasnone",
    ];

    let mut pager = Pager::new().unwrap();

    for line in LINES {
        write!(pager, "{}", line).unwrap();
    }

    assert_eq!(
        pager.formatted_lines,
        vec![
            "This line has trailing whitespace           This has leading whitespace",
            "   This has whitespace on both sides   Andthishasnone"
        ]
    );
}

#[test]
fn test_incrementally_push() {
    const LINES: [&str; 4] = [
        "this is a line",
        " and this is another",
        " and this is yet another\n",
        "and this should be on a newline",
    ];

    let mut pager = Pager::new().unwrap();

    pager.push_str(LINES[0]);

    assert_eq!(pager.lines, LINES[0].to_owned());
    assert_eq!(pager.formatted_lines, vec![LINES[0].to_owned()]);

    pager.push_str(LINES[1]);

    let line = LINES[..2].join("");
    assert_eq!(pager.lines, line);
    assert_eq!(pager.formatted_lines, vec![line]);

    pager.push_str(LINES[2]);

    let mut line = LINES[..3].join("");
    assert_eq!(pager.lines, line);

    line.pop();
    assert_eq!(pager.formatted_lines, vec![line]);

    pager.push_str(LINES[3]);

    let joined = LINES.join("");
    assert_eq!(pager.lines, joined);
    assert_eq!(
        pager.formatted_lines,
        joined
            .lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
    );
}

#[test]
fn test_multiple_newlines() {
    const TEST: &str = "This\n\n\nhas many\n newlines\n";

    let mut pager = Pager::new().unwrap();

    pager.push_str(TEST);

    assert_eq!(pager.lines, TEST.to_owned());
    assert_eq!(
        pager.formatted_lines,
        TEST.lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
    );

    pager.set_text(TEST);

    assert_eq!(pager.lines, TEST.to_owned());
    assert_eq!(
        pager.formatted_lines,
        TEST.lines()
            .map(ToString::to_string)
            .collect::<Vec<String>>()
    );
}

#[test]
fn test_floating_newline_write() {
    const TEST: &str = "This is a line with a bunch of\nin between\nbut not at the end";
    let mut pager = Pager::new().unwrap();
    write!(pager, "{}", TEST).unwrap();
    assert_eq!(
        pager.formatted_lines,
        vec![
            "This is a line with a bunch of".to_string(),
            "in between".to_string(),
            "but not at the end".to_owned()
        ]
    );
    assert_eq!(pager.lines, TEST.to_string());
}

// Test exit callbacks function
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
#[test]
fn test_exit_callback() {
    use std::sync::atomic::Ordering;
    use std::sync::{atomic::AtomicBool, Arc};
    let mut pager = Pager::new().unwrap();
    let exited = Arc::new(AtomicBool::new(false));
    let exited_within_callback = exited.clone();
    pager.add_exit_callback(move || exited_within_callback.store(true, Ordering::Relaxed));
    pager.exit();

    assert!(exited.load(Ordering::Relaxed));
}

// Test wrapping functions
#[test]
fn test_wrap_str() {
    let test = {
        let mut line = String::with_capacity(200);
        for _ in 1..=200 {
            line.push('#');
        }
        line
    };
    let result = wrap_str(&test, 80);
    assert_eq!(result.len(), 3);
    assert_eq!(
        (80, 80, 40),
        (result[0].len(), result[1].len(), result[2].len()),
    );
}

#[test]
fn test_rewrap() {
    let test = {
        let mut line = String::with_capacity(200);
        for _ in 1..=200 {
            line.push('#');
        }
        line
    };

    let mut line = crate::wrap_str(&test, 80);

    assert_eq!(line.len(), 3);
    assert_eq!((80, 80, 40), (line[0].len(), line[1].len(), line[2].len()),);

    rewrap(&mut line, 100);

    assert_eq!(line.len(), 3);
    // No change, since it's already in a good optimal state
    assert_eq!((80, 80, 40), (line[0].len(), line[1].len(), line[2].len()));
}
