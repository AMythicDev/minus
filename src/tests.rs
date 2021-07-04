use super::{rewrap, wrap_str, Pager};
use std::fmt::Write;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc};

// Test the implementation of std::fmt::Write on Pager
#[test]
fn test_writeln() {
    const TEST: &str = "This is a line";
    let mut pager = Pager::new().unwrap();
    writeln!(pager, "{}", TEST).unwrap();
    assert_eq!(pager.wrap_lines, vec![vec![TEST]]);
}

#[test]
fn test_write() {
    const TEST: &str = "This is a line";
    let mut pager = Pager::new().unwrap();
    write!(pager, "{}", TEST).unwrap();
    let res: Vec<Vec<String>> = Vec::new();
    assert_eq!(pager.wrap_lines, res);
    assert_eq!(pager.lines, TEST.to_string())
}

#[test]
fn test_sequential_write() {
    const TEXT1: &str = "This is a line.";
    const TEXT2: &str = " This is a follow up line";
    let mut pager = Pager::new().unwrap();
    write!(pager, "{}", TEXT1).unwrap();
    write!(pager, "{}", TEXT2).unwrap();
    let res: Vec<Vec<String>> = Vec::new();
    assert_eq!(pager.wrap_lines, res);
    assert_eq!(pager.lines, TEXT1.to_string() + TEXT2)
}

#[test]
fn test_sequential_writeln() {
    const TEXT1: &str = "This is a line.";
    const TEXT2: &str = " This is a follow up line";
    let mut pager = Pager::new().unwrap();
    writeln!(pager, "{}", TEXT1).unwrap();
    writeln!(pager, "{}", TEXT2).unwrap();
    assert_eq!(
        pager.wrap_lines,
        vec![vec![TEXT1.to_string()], vec![TEXT2.to_string()]]
    );
}

#[test]
fn test_floating_newline_write() {
    const TEST: &str = "This is a line with a bunch of\nin between\nbut not at the end";
    let mut pager = Pager::new().unwrap();
    write!(pager, "{}", TEST).unwrap();
    assert_eq!(
        pager.wrap_lines,
        vec![
            vec!["This is a line with a bunch of".to_string()],
            vec!["in between".to_string()]
        ]
    );
    assert_eq!(pager.lines, "but not at the end".to_string());
}

// Test exit callbacks function
#[cfg(any(feature = "tokio_lib", feature = "async_std_lib"))]
#[test]
fn test_exit_callback() {
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
            line.push('#')
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
            line.push('#')
        }
        line
    };
    let mut line: Vec<String> = textwrap::wrap(&test, 80)
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    assert_eq!(line.len(), 3);
    assert_eq!((80, 80, 40), (line[0].len(), line[1].len(), line[2].len()),);

    rewrap(&mut line, 100);

    assert_eq!(line.len(), 2);
    assert_eq!((100, 100), (line[0].len(), line[1].len()));
}
