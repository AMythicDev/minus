use super::{rewrap, wrap_str, Pager};
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc};

#[test]
fn test_writeln() {
    const TEST: &str = "This is a line";
    let mut pager = Pager::new().unwrap();
    writeln!(pager, "{}", TEST).unwrap();
    assert_eq!(pager.wrap_lines, vec![vec![TEST]]);
    assert_eq!(&pager.lines, "");
}

#[test]
fn test_write() {
    const TEST: &str = "This is a line";
    let mut pager = Pager::new().unwrap();
    write!(pager, "{}", TEST).unwrap();
    let res: Vec<Vec<String>> = Vec::new();
    assert_eq!(pager.wrap_lines, res);
    assert_eq!(&pager.lines, TEST);
}

#[cfg(feature = "tokio_lib")]
#[test]
fn test_exit_callback() {
    let mut pager = Pager::new().unwrap();
    let exited = Arc::new(AtomicBool::new(false));
    let exited_within_callback = exited.clone();
    pager.add_exit_callback(move || exited_within_callback.store(true, Ordering::Relaxed));
    pager.exit();

    assert!(exited.load(Ordering::Relaxed));
}

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
