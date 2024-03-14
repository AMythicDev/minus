mod unterminated {
    use crate::screen::{format_text_block, FormatOpts, Rows};

    const fn get_append_opts_template(text: &str) -> FormatOpts<Rows> {
        FormatOpts {
            buffer: Vec::new(),
            text,
            attachment: None,
            #[cfg(feature = "search")]
            search_term: &None,
            lines_count: 0,
            formatted_lines_count: 0,
            cols: 80,
            line_numbers: crate::LineNumbers::Disabled,
            prev_unterminated: 0,
            line_wrapping: true,
        }
    }

    #[test]
    fn test_single_no_endline() {
        let append_style = format_text_block(get_append_opts_template("This is a line"));
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_single_endline() {
        let append_style = format_text_block(get_append_opts_template("This is a line\n"));
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_single_multi_newline() {
        let append_style = format_text_block(get_append_opts_template(
            "This is a line\nThis is another line\nThis is third line",
        ));
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_single_multi_endline() {
        let append_style = format_text_block(get_append_opts_template(
            "This is a line\nThis is another line\n",
        ));
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_single_line_wrapping() {
        let mut fs = get_append_opts_template("This is a quite lengthy line");
        fs.cols = 20;
        let append_style = format_text_block(fs);
        assert_eq!(2, append_style.num_unterminated);
    }

    #[test]
    fn test_single_mid_newline_wrapping() {
        let mut fs = get_append_opts_template(
            "This is a quite lengthy line\nIt has three lines\nThis is
third line",
        );
        fs.cols = 20;
        let append_style = format_text_block(fs);
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_single_endline_wrapping() {
        let mut fs = get_append_opts_template(
            "This is a quite lengthy line\nIt has three lines\nThis is
third line\n",
        );
        fs.cols = 20;
        let append_style = format_text_block(fs);
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_no_endline() {
        let append_style = format_text_block(get_append_opts_template("This is a line. "));
        assert_eq!(1, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is another line");
        fs.prev_unterminated = append_style.num_unterminated;
        fs.attachment = Some("This is a line. ");

        let append_style = format_text_block(fs);
        assert_eq!(1, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_endline() {
        let append_style = format_text_block(get_append_opts_template("This is a line. "));
        assert_eq!(1, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is another line\n");
        fs.prev_unterminated = append_style.num_unterminated;
        fs.attachment = Some("This is a line. ");

        let append_style = format_text_block(fs);
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_multiple_newline() {
        let append_style = format_text_block(get_append_opts_template("This is a line\n"));
        assert_eq!(0, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is another line\n");
        fs.lines_count = 1;
        fs.formatted_lines_count = 1;
        fs.attachment = None;

        let append_style = format_text_block(fs);
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping() {
        let mut fs = get_append_opts_template("This is a line. This is second line. ");
        fs.cols = 20;
        let append_style = format_text_block(fs);
        assert_eq!(2, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is another line\n");
        fs.cols = 20;
        fs.prev_unterminated = append_style.num_unterminated;
        fs.attachment = Some("This is a line. This is second line");

        let append_style = format_text_block(fs);
        assert_eq!(0, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping_continued() {
        let mut fs = get_append_opts_template("This is a line. This is second line. ");
        fs.cols = 20;
        let append_style = format_text_block(fs);
        assert_eq!(2, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is third line");
        fs.cols = 20;
        fs.prev_unterminated = append_style.num_unterminated;
        fs.attachment = Some("This is a line. This is second line. ");

        let append_style = format_text_block(fs);
        assert_eq!(3, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping_last_continued() {
        let mut fs = get_append_opts_template("This is a line.\nThis is second line. ");
        fs.cols = 20;
        let append_style = format_text_block(fs);
        assert_eq!(1, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is third line.");
        fs.cols = 20;
        fs.prev_unterminated = append_style.num_unterminated;
        fs.attachment = Some("This is second line. ");
        fs.lines_count = 1;
        fs.formatted_lines_count = 2;

        let append_style = format_text_block(fs);

        assert_eq!(2, append_style.num_unterminated);
    }

    #[test]
    fn test_multi_wrapping_additive() {
        let mut fs = get_append_opts_template("This is a line. ");
        fs.cols = 20;
        let append_style = format_text_block(fs);
        assert_eq!(1, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is second line. ");
        fs.cols = 20;
        fs.prev_unterminated = append_style.num_unterminated;
        fs.attachment = Some("This is a line. ");

        let append_style = format_text_block(fs);
        assert_eq!(2, append_style.num_unterminated);

        let mut fs = get_append_opts_template("This is third line");
        fs.cols = 20;
        fs.prev_unterminated = append_style.num_unterminated;
        fs.attachment = Some("This is a line. This is second line. ");
        let append_style = format_text_block(fs);

        assert_eq!(3, append_style.num_unterminated);
    }
}
