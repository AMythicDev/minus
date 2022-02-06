// Test the implementation of std::fmt::Write on Pager
mod fmt_write {
    use crate::{Event, Pager};
    use std::fmt::Write;

    #[test]
    fn pager_writeln() {
        const TEST: &str = "This is a line";
        let mut pager = Pager::new();
        writeln!(pager, "{}", TEST).unwrap();
        while let Ok(Event::AppendData(text)) = pager.rx.try_recv() {
            if text != "\n" {
                assert_eq!(text, TEST.to_string());
            }
        }
    }

    #[test]
    fn test_write() {
        const TEST: &str = "This is a line";
        let mut pager = Pager::new();
        write!(pager, "{}", TEST).unwrap();
        while let Ok(Event::AppendData(text)) = pager.rx.try_recv() {
            assert_eq!(text, TEST.to_string());
        }
    }
}

mod pager_append_str {
    use crate::PagerState;
    #[test]
    fn sequential_append_str() {
        const TEXT1: &str = "This is a line.";
        const TEXT2: &str = " This is a follow up line";
        let mut ps = PagerState::new().unwrap();
        ps.append_str(TEXT1);
        ps.append_str(TEXT2);
        assert_eq!(ps.formatted_lines, vec![format!("{}{}", TEXT1, TEXT2)]);
        assert_eq!(ps.lines, TEXT1.to_string() + TEXT2);
    }

    #[test]
    fn append_sequential_lines() {
        const TEXT1: &str = "This is a line.";
        const TEXT2: &str = " This is a follow up line";
        let mut ps = PagerState::new().unwrap();
        ps.append_str(&(TEXT1.to_string() + "\n"));
        ps.append_str(&(TEXT2.to_string() + "\n"));

        assert_eq!(
            ps.formatted_lines,
            vec![TEXT1.to_string(), TEXT2.to_string()]
        );
    }

    #[test]
    fn crlf_write() {
        const LINES: [&str; 4] = [
            "hello,\n",
            "this is ",
            "a test\r\n",
            "of weird line endings",
        ];

        let mut ps = PagerState::new().unwrap();

        for line in LINES {
            ps.append_str(line);
        }

        assert_eq!(
            ps.formatted_lines,
            vec![
                "hello,".to_string(),
                "this is a test".to_string(),
                "of weird line endings".to_string()
            ]
        );
    }

    #[test]
    fn unusual_whitespace() {
        const LINES: [&str; 4] = [
            "This line has trailing whitespace      ",
            "     This has leading whitespace\n",
            "   This has whitespace on both sides   ",
            "Andthishasnone",
        ];

        let mut ps = PagerState::new().unwrap();

        for line in LINES {
            ps.append_str(line);
        }

        assert_eq!(
            ps.formatted_lines,
            vec![
                "This line has trailing whitespace           This has leading whitespace",
                "   This has whitespace on both sides   Andthishasnone"
            ]
        );
    }

    #[test]
    fn incremental_append() {
        const LINES: [&str; 4] = [
            "this is a line",
            " and this is another",
            " and this is yet another\n",
            "and this should be on a newline",
        ];

        let mut ps = PagerState::new().unwrap();

        ps.append_str(LINES[0]);

        assert_eq!(ps.lines, LINES[0].to_owned());
        assert_eq!(ps.formatted_lines, vec![LINES[0].to_owned()]);

        ps.append_str(LINES[1]);

        let line = LINES[..2].join("");
        assert_eq!(ps.lines, line);
        assert_eq!(ps.formatted_lines, vec![line]);

        ps.append_str(LINES[2]);

        let mut line = LINES[..3].join("");
        assert_eq!(ps.lines, line);

        line.pop();
        assert_eq!(ps.formatted_lines, vec![line]);

        ps.append_str(LINES[3]);

        let joined = LINES.join("");
        assert_eq!(ps.lines, joined);
        assert_eq!(
            ps.formatted_lines,
            joined
                .lines()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        );
    }

    #[test]
    fn multiple_newlines() {
        const TEST: &str = "This\n\n\nhas many\n newlines\n";

        let mut ps = PagerState::new().unwrap();

        ps.append_str(TEST);

        assert_eq!(ps.lines, TEST.to_owned());
        assert_eq!(
            ps.formatted_lines,
            TEST.lines()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        );

        ps.lines = TEST.to_string();
        ps.format_lines();

        assert_eq!(ps.lines, TEST.to_owned());
        assert_eq!(
            ps.formatted_lines,
            TEST.lines()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
        );
    }

    #[test]
    fn append_floating_newline() {
        const TEST: &str = "This is a line with a bunch of\nin between\nbut not at the end";
        let mut ps = PagerState::new().unwrap();
        ps.append_str(TEST);
        assert_eq!(
            ps.formatted_lines,
            vec![
                "This is a line with a bunch of".to_string(),
                "in between".to_string(),
                "but not at the end".to_owned()
            ]
        );
        assert_eq!(ps.lines, TEST.to_string());
    }
}

// Test exit callbacks function
#[cfg(feature = "async_output")]
#[test]
fn exit_callback() {
    use crate::PagerState;
    use std::sync::atomic::Ordering;
    use std::sync::{atomic::AtomicBool, Arc};

    let mut ps = PagerState::new().unwrap();
    let exited = Arc::new(AtomicBool::new(false));
    let exited_within_callback = exited.clone();
    ps.exit_callbacks.push(Box::new(move || {
        exited_within_callback.store(true, Ordering::Relaxed);
    }));
    ps.exit();

    assert!(exited.load(Ordering::Relaxed));
}

mod wrapping {
    // Test wrapping functions
    #[test]
    fn wrap_str() {
        let test = {
            let mut line = String::with_capacity(200);
            for _ in 1..=200 {
                line.push('#');
            }
            line
        };
        let result = crate::wrap_str(&test, 80);
        assert_eq!(result.len(), 3);
        assert_eq!(
            (80, 80, 40),
            (result[0].len(), result[1].len(), result[2].len()),
        );
    }

    #[test]
    fn rewrap() {
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

        crate::rewrap(&mut line, 100);

        assert_eq!(line.len(), 3);
        // No change, since it's already in a good optimal state
        assert_eq!((80, 80, 40), (line[0].len(), line[1].len(), line[2].len()));
    }
}

mod emit_events {
    // Check functions emit correct events on functin calls
    use crate::{events::Event, ExitStrategy, LineNumbers, Pager};

    const TEST_STR: &str = "This is sample text";
    #[test]
    fn set_text() {
        let pager = Pager::new();
        pager.set_text(TEST_STR).unwrap();
        assert_eq!(
            Event::SetData(TEST_STR.to_string()),
            pager.rx.try_recv().unwrap()
        );
    }

    #[test]
    fn push_str() {
        let pager = Pager::new();
        pager.push_str(TEST_STR).unwrap();
        assert_eq!(
            Event::AppendData(TEST_STR.to_string()),
            pager.rx.try_recv().unwrap()
        );
    }

    #[test]
    fn set_prompt() {
        let pager = Pager::new();
        pager.set_prompt(TEST_STR).unwrap();
        assert_eq!(
            Event::SetPrompt(TEST_STR.to_string()),
            pager.rx.try_recv().unwrap()
        );
    }

    #[test]
    fn send_message() {
        let pager = Pager::new();
        pager.send_message(TEST_STR).unwrap();
        assert_eq!(
            Event::SendMessage(TEST_STR.to_string()),
            pager.rx.try_recv().unwrap()
        );
    }

    #[test]
    #[cfg(feature = "static_output")]
    fn set_run_no_overflow() {
        let pager = Pager::new();
        pager.set_run_no_overflow(false).unwrap();
        assert_eq!(Event::SetRunNoOverflow(false), pager.rx.try_recv().unwrap());
    }

    #[test]
    fn set_line_numbers() {
        let pager = Pager::new();
        pager.set_line_numbers(LineNumbers::Enabled).unwrap();
        assert_eq!(
            Event::SetLineNumbers(LineNumbers::Enabled),
            pager.rx.try_recv().unwrap()
        );
    }

    #[test]
    fn set_exit_strategy() {
        let pager = Pager::new();
        pager.set_exit_strategy(ExitStrategy::PagerQuit).unwrap();
        assert_eq!(
            Event::SetExitStrategy(ExitStrategy::PagerQuit),
            pager.rx.try_recv().unwrap()
        );
    }

    #[test]
    fn add_exit_callback() {
        let func = Box::new(|| println!("Hello"));
        let pager = Pager::new();
        pager.add_exit_callback(func.clone()).unwrap();

        assert_eq!(Event::AddExitCallback(func), pager.rx.try_recv().unwrap());
    }
}
