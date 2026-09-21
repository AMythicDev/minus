use std::{fmt::Write, sync::Arc};

use parking_lot::Mutex;

use super::{DataSource, InMemorySource};

#[test]
fn empty_source() {
    let src = InMemorySource::new();
    assert_eq!(src.line_count(), 0);
    assert_eq!(src.line(0), None);
    assert!(src.last_line_terminated());
    assert!(!src.is_complete());
}

#[test]
fn append_starts_new_line_after_terminated() {
    let mut src = InMemorySource::new();
    src.append("abc\n");
    assert_eq!(src.line_count(), 1);
    assert_eq!(src.line(0).as_deref(), Some("abc"));
    assert!(src.last_line_terminated());

    src.append("def\n");
    assert_eq!(src.line_count(), 2);
    assert_eq!(src.line(1).as_deref(), Some("def"));
    assert!(src.last_line_terminated());
}

#[test]
fn append_merges_into_unterminated_line() {
    let mut src = InMemorySource::new();
    src.append("abc");
    assert_eq!(src.line_count(), 1);
    assert_eq!(src.line(0).as_deref(), Some("abc"));
    assert!(!src.last_line_terminated());

    // The incoming text is part of the unterminated last line
    src.append("def\n");
    assert_eq!(src.line_count(), 1);
    assert_eq!(src.line(0).as_deref(), Some("abcdef"));
    assert!(src.last_line_terminated());

    src.append("ghi\n");
    assert_eq!(src.line_count(), 2);
    assert_eq!(src.line(1).as_deref(), Some("ghi"));
}

#[test]
fn append_multiline_merges_first_line() {
    let mut src = InMemorySource::new();
    src.append("abc");
    src.append("def\nghi\n");
    assert_eq!(src.line_count(), 2);
    assert_eq!(src.line(0).as_deref(), Some("abcdefghi"));
    assert_eq!(src.line(1).as_deref(), Some("ghi"));
    assert!(src.last_line_terminated());
}

#[test]
fn append_empty_text_is_noop() {
    let mut src = InMemorySource::new();
    src.append("abc\n");
    src.append("");
    assert_eq!(src.line_count(), 1);
    assert_eq!(src.line(0).as_deref(), Some("abc"));
    assert!(src.last_line_terminated());
}

#[test]
fn line_strips_line_endings_like_str_lines() {
    let src = InMemorySource::from("a\r\nb\nc");
    assert_eq!(src.line(0).as_deref(), Some("a"));
    assert_eq!(src.line(1).as_deref(), Some("b"));
    assert_eq!(src.line(2).as_deref(), Some("c"));
    assert_eq!(src.line(3), None);
    assert_eq!(src.line_count(), 3);
    assert!(!src.last_line_terminated());
}

#[test]
fn line_handles_empty_lines() {
    let src = InMemorySource::from("\n\n");
    assert_eq!(src.line_count(), 2);
    assert_eq!(src.line(0).as_deref(), Some(""));
    assert_eq!(src.line(1).as_deref(), Some(""));

    let src = InMemorySource::from("a\n\nb\n");
    assert_eq!(src.line_count(), 3);
    assert_eq!(src.line(0).as_deref(), Some("a"));
    assert_eq!(src.line(1).as_deref(), Some(""));
    assert_eq!(src.line(2).as_deref(), Some("b"));
}

#[test]
fn line_handles_lone_carriage_return() {
    // A '\r' not followed by a '\n' is not a line terminator
    let src = InMemorySource::from("a\rb\n");
    assert_eq!(src.line_count(), 1);
    assert_eq!(src.line(0).as_deref(), Some("a\rb"));
}

#[test]
fn line_handles_multibyte_utf8() {
    let src = InMemorySource::from("▲▼\n日本語\n");
    assert_eq!(src.line(0).as_deref(), Some("▲▼"));
    assert_eq!(src.line(1).as_deref(), Some("日本語"));
    assert_eq!(src.line(2), None);
}

#[test]
fn line_matches_str_lines_randomized() {
    // Deterministic pseudo-random text covering varied line content, line
    // endings and a final unterminated line
    let mut text = String::new();
    let mut state = 0x5eed_1234_u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state
    };
    for _ in 0..500 {
        let len = (next() % 20) as usize;
        for _ in 0..len {
            let ch = match next() % 5 {
                0 => 'a',
                1 => 'Z',
                2 => '▲',
                3 => ' ',
                _ => '\n',
            };
            text.push(ch);
        }
        // Randomly terminate the block so that the last line is sometimes
        // unterminated
        if next() % 4 == 0 {
            text.push('\n');
        }
    }

    let src = InMemorySource::from(&text);
    let expected = text.lines().collect::<Vec<_>>();
    assert_eq!(src.line_count(), expected.len());
    for (idx, line) in expected.iter().enumerate() {
        assert_eq!(src.line(idx).as_deref(), Some(*line));
    }
}

#[test]
fn replace_resets_everything() {
    let mut src = InMemorySource::from("old\ntext\n");
    src.finish();
    assert!(src.is_complete());

    src.replace("new\n");
    assert_eq!(src.line_count(), 1);
    assert_eq!(src.line(0).as_deref(), Some("new"));
    assert!(src.last_line_terminated());
    // Replacing voids the previous completion promise
    assert!(!src.is_complete());
}

#[test]
fn finish_marks_complete() {
    let mut src = InMemorySource::from("abc\n");
    assert!(!src.is_complete());
    src.finish();
    assert!(src.is_complete());
}

#[test]
fn from_impls() {
    let src = InMemorySource::from("a\nb\n");
    assert_eq!(src.line_count(), 2);

    let src = InMemorySource::from(String::from("a\nb\n"));
    assert_eq!(src.line_count(), 2);

    // Each element is appended as raw text in order
    let src = ["a\n", "b\n"]
        .into_iter()
        .map(String::from)
        .collect::<InMemorySource>();
    assert_eq!(src.line_count(), 2);
    assert_eq!(src.line(0).as_deref(), Some("a"));
    assert_eq!(src.line(1).as_deref(), Some("b"));
}

#[test]
fn fmt_write_appends() {
    let mut src = InMemorySource::new();
    writeln!(src, "Hello {}", "World").unwrap();
    writeln!(src, "second line").unwrap();
    assert_eq!(src.line_count(), 2);
    assert_eq!(src.line(0).as_deref(), Some("Hello World"));
    assert_eq!(src.line(1).as_deref(), Some("second line"));
}

#[test]
fn shared_source_through_mutex_lock() {
    let source = Arc::new(Mutex::new(InMemorySource::new()));
    let dyn_source: Box<dyn DataSource> = Box::new(source.clone());

    assert_eq!(dyn_source.line_count(), 0);
    assert_eq!(dyn_source.line(0), None);
    assert!(dyn_source.last_line_terminated());

    source.lock().append("abc\n");
    assert_eq!(dyn_source.line_count(), 1);
    assert_eq!(dyn_source.line(0).as_deref(), Some("abc"));

    // Merging through the lock is visible through the trait object
    source.lock().append("def");
    assert_eq!(dyn_source.line_count(), 1);
    assert_eq!(dyn_source.line(0).as_deref(), Some("abcdef"));
    assert!(!dyn_source.last_line_terminated());
}

#[test]
fn shared_source_through_arc_zero_copy() {
    let source = Arc::new(InMemorySource::from("abc\ndef\n"));
    let dyn_source: Box<dyn DataSource> = Box::new(source.clone());

    assert_eq!(dyn_source.line_count(), 2);
    assert_eq!(dyn_source.line(0).as_deref(), Some("abc"));
    assert_eq!(dyn_source.line(1).as_deref(), Some("def"));
    assert!(dyn_source.last_line_terminated());
    assert!(!dyn_source.is_complete());
}
