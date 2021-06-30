use super::Pager;
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
pub fn test_exit_callback() {
    let mut pager = Pager::new().unwrap();
    let exited = Arc::new(AtomicBool::new(false));
    let exited_within_callback = exited.clone();
    pager.add_exit_callback(move || exited_within_callback.store(true, Ordering::Relaxed));
    pager.exit();

    assert!(exited.load(Ordering::Relaxed));
}
