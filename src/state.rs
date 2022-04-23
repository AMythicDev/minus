#[cfg(feature = "search")]
use crate::minus_core::search::{self, SearchMode};
use crate::{error::TermError, input, wrap_str, ExitStrategy, LineNumbers};
use crossterm::{terminal, tty::IsTty};
#[cfg(feature = "search")]
use std::collections::BTreeSet;
use std::io::stdout;

/// Holds all information and configuration about the pager during
/// its un time.
///
/// This type is exposed so that end-applications can implement the
/// [`InputClassifier`](input::InputClassifier) trait which requires the `PagerState` to be passed
/// as a parameter
///
/// Various fields are made public so that their values can be accessed while implementing the
/// trait.
#[allow(clippy::module_name_repetitions)]
pub struct PagerState {
    /// The text the pager has been told to be displayed
    pub(crate) lines: String,
    /// The output, flattened and formatted into the lines that should be displayed
    pub(crate) formatted_lines: Vec<String>,
    /// Configuration for line numbers. See [`LineNumbers`]
    pub line_numbers: LineNumbers,
    /// Unterminated lines
    /// Keeps track of the number of lines at the last of [PagerState::formatted_lines] which are
    /// not terminated by a newline
    pub(crate) unterminated: usize,
    /// The prompt displayed at the bottom wrapped to available terminal width
    pub(crate) prompt: String,
    /// The input classifier to be called when a input is detected
    pub(crate) input_classifier: Box<dyn input::InputClassifier + Sync + Send>,
    /// Functions to run when the pager quits
    pub(crate) exit_callbacks: Vec<Box<dyn FnMut() + Send + Sync + 'static>>,
    /// The behaviour to do when user quits the program using `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    pub(crate) exit_strategy: ExitStrategy,
    /// Any message to display to the user at the prompt
    /// The first element contains the actual message, while the second element tells
    /// whether the message has changed since the last display.
    pub(crate) message: Option<String>,
    /// The prompt that should be displayed to the user, formatted with the
    /// current search index and number of matches (if the search feature is enabled),
    /// and the current numbers inputted to scroll
    pub(crate) displayed_prompt: String,
    /// The upper bound of scrolling.
    ///
    /// This is useful for keeping track of the range of lines which are currently being displayed on
    /// the terminal.
    /// When `rows - 1` is added to the `upper_mark`, it gives the lower bound of scroll.
    ///
    /// For example if there are 10 rows is a terminal and the data to display has 50 lines in it/
    /// If the `upper_mark` is 15, then the first row of the terminal is the 16th line of the data
    /// and last row is the 24th line of the data.
    pub upper_mark: usize,
    /// Do we want to page if there is no overflow
    #[cfg(feature = "static_output")]
    pub(crate) run_no_overflow: bool,
    /// Stores the most recent search term
    #[cfg(feature = "search")]
    pub(crate) search_term: Option<regex::Regex>,
    /// Direction of search
    ///
    /// See [`SearchMode`] for available options
    #[cfg(feature = "search")]
    #[cfg_attr(docsrs, cfg(feature = "search"))]
    pub search_mode: SearchMode,
    /// Lines where searches have a match
    /// In order to avoid duplicate entries of lines, we keep it in a [`BTreeSet`]
    #[cfg(feature = "search")]
    pub(crate) search_idx: BTreeSet<usize>,
    /// Index of search item currently in focus
    /// It should be 0 even when no search is in action
    #[cfg(feature = "search")]
    pub(crate) search_mark: usize,
    /// Available rows in the terminal
    pub rows: usize,
    /// Available columns in the terminal
    pub cols: usize,
    /// This variable helps in scrolling more than one line at a time
    /// It keeps track of all the numbers that have been entered by the user
    /// untill any of `j`, `k`, `G`, `Up` or `Down` is pressed
    pub prefix_num: String,
}

impl PagerState {
    pub(crate) fn new() -> Result<Self, TermError> {
        let (rows, cols);

        if cfg!(test) {
            // In tests, set  number of columns to 80 and rows to 10
            cols = 80;
            rows = 10;
        } else if stdout().is_tty() {
            // If a proper terminal is present, get size and set it
            let size = terminal::size()?;
            cols = size.0 as usize;
            rows = size.1 as usize;
        } else {
            // For other cases beyond control
            cols = 1;
            rows = 1;
        };

        let mut ret_pager = Self {
            lines: String::with_capacity(u16::MAX.into()),
            formatted_lines: Vec::with_capacity(u16::MAX.into()),
            line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            unterminated: 0,
            prompt: "minus".to_owned(),
            exit_strategy: ExitStrategy::ProcessQuit,
            input_classifier: Box::new(input::DefaultInputClassifier {}),
            exit_callbacks: Vec::with_capacity(5),
            message: None,
            displayed_prompt: String::new(),
            #[cfg(feature = "static_output")]
            run_no_overflow: false,
            #[cfg(feature = "search")]
            search_term: None,
            #[cfg(feature = "search")]
            search_mode: SearchMode::default(),
            #[cfg(feature = "search")]
            search_idx: BTreeSet::new(),
            #[cfg(feature = "search")]
            search_mark: 0,
            // Just to be safe in tests, keep at 1x1 size
            cols,
            rows,
            prefix_num: String::new(),
        };

        ret_pager.format_prompt();
        Ok(ret_pager)
    }

    pub(crate) fn num_lines(&self) -> usize {
        self.formatted_lines.len()
    }

    /// Formats the given `line`
    ///
    /// - `line_numbers` tells whether to format the line with line numbers.
    /// - `len_line_number` is the length of the number of lines in [`PagerState::lines`] as in a string.
    ///     For example, this will be 2 if number of lines in [`PagerState::lines`] is 50 and 3 if
    ///     number of lines in [`PagerState::lines`] is 500. This is used for calculating the padding
    ///     of each displayed line.
    /// - `idx` is the position index where the line is placed in [`PagerState::lines`].
    /// - `formatted_idx` is the position index where the line will be placed in the resulting
    ///    [`PagerState::formatted_lines`]
    pub(crate) fn formatted_line(
        &self,
        line: &str,
        len_line_number: usize,
        idx: usize,
        #[cfg(feature = "search")] formatted_idx: usize,
        #[cfg(feature = "search")] search_idx: &mut BTreeSet<usize>,
    ) -> Vec<String> {
        let line_numbers = matches!(
            self.line_numbers,
            LineNumbers::Enabled | LineNumbers::AlwaysOn
        );

        if line_numbers {
            // Padding is the space that the actual line text will be shifted to accomodate for
            // in line numbers. This is equal to:-
            // 1 for initial space + len_line_number + 1 for `.` sign and + 1 for the followup space
            //
            // We reduce this from the number of available columns as this space cannot be used for
            // actual line display when wrapping the lines
            let padding = len_line_number + 3;
            #[cfg_attr(not(feature = "search"), allow(unused_mut))]
            #[cfg_attr(not(feature = "search"), allow(unused_variables))]
            wrap_str(line, self.cols.saturating_sub(padding))
                .into_iter()
                .enumerate()
                .map(|(wrap_idx, mut row)| {
                    #[cfg(feature = "search")]
                    if let Some(st) = self.search_term.as_ref() {
                        // highlight the lines with matching search terms
                        // If a match is found, add this line's index to PagerState::search_idx
                        let (hrow, is_match) = search::highlight_line_matches(&row, st);
                        if is_match {
                            search_idx.insert(formatted_idx + wrap_idx);
                        }
                        row = hrow;
                    }

                    if cfg!(not(test)) {
                        format!(
                            " {bold}{number: >len$}.{reset} {row}",
                            bold = crossterm::style::Attribute::Bold,
                            number = idx + 1,
                            len = len_line_number,
                            reset = crossterm::style::Attribute::Reset,
                            row = row
                        )
                    } else {
                        // In tests, we don't care about ANSI sequences for cool looking line numbers
                        // hence we don't include them in tests. It just makes testing more difficult
                        format!(
                            " {number: >len$}. {row}",
                            number = idx + 1,
                            len = len_line_number,
                            row = row
                        )
                    }
                })
                .collect::<Vec<String>>()
        } else {
            #[cfg_attr(not(feature = "search"), allow(unused_variables))]
            wrap_str(line, self.cols)
                .iter()
                .enumerate()
                .map(|(wrap_idx, row)| {
                    #[cfg(feature = "search")]
                    {
                        self.search_term.as_ref().map_or_else(
                            || row.to_string(),
                            |st| {
                                // highlight the lines with matching search terms
                                // If a match is found, add this line's index to PagerState::search_idx
                                let (hrow, is_match) = search::highlight_line_matches(row, st);
                                if is_match {
                                    search_idx.insert(formatted_idx + wrap_idx);
                                }
                                hrow
                            },
                        )
                    }
                    #[cfg(not(feature = "search"))]
                    row.to_string()
                })
                .collect::<Vec<String>>()
        }
    }

    pub(crate) fn format_lines(&mut self) {
        // Keep it for the record and don't call it unless it is really necessory as this is kinda
        // expensive
        let line_count = self.lines.lines().count();

        // Calculate len_line_number. This will be 2 if line_count if 50 and 3 if line_count is 100.
        let len_line_number = line_count.to_string().len();

        // Search idx, this will get filled by the self.formatted_line function
        // we will later set this to self.search_idx
        #[cfg(feature = "search")]
        let mut search_idx = BTreeSet::new();
        let mut formatted_idx = 0;

        self.formatted_lines = self
            .lines
            .lines()
            .enumerate()
            .flat_map(|(idx, line)| {
                let new_line = self.formatted_line(
                    line,
                    len_line_number,
                    idx,
                    #[cfg(feature = "search")]
                    formatted_idx,
                    #[cfg(feature = "search")]
                    &mut search_idx,
                );
                formatted_idx += new_line.len();
                new_line
            })
            .collect::<Vec<String>>();

        #[cfg(feature = "search")]
        {
            self.search_idx = search_idx;
        }

        self.format_prompt();
    }

    /// Reformat the inputted prompt to how it should be displayed
    pub(crate) fn format_prompt(&mut self) {
        const SEARCH_BG: &str = "\x1b[34m";
        const INPUT_BG: &str = "\x1b[33m";

        // Allocate the string. Add extra space in case for the
        // ANSI escape things if we do have characters typed and search showing
        let mut format_string = String::with_capacity(self.cols + (SEARCH_BG.len() * 2) + 4);

        // Get the string that will contain the search index/match indicator
        #[cfg(feature = "search")]
        let mut search_str = String::new();
        #[cfg(feature = "search")]
        if !self.search_idx.is_empty() {
            search_str.push(' ');
            search_str.push_str(&(self.search_mark + 1).to_string());
            search_str.push('/');
            search_str.push_str(&self.search_idx.len().to_string());
            search_str.push(' ');
        }

        // And get the string that will contain the prefix_num
        let mut prefix_str = String::new();
        if !self.prefix_num.is_empty() {
            prefix_str.push(' ');
            prefix_str.push_str(&self.prefix_num);
            prefix_str.push(' ');
        }

        // And lastly, the string that contains the prompt or msg
        let prompt_str = self.message.as_ref().unwrap_or(&self.prompt);

        #[cfg(feature = "search")]
        let search_len = search_str.len();
        #[cfg(not(feature = "search"))]
        let search_len = 0;

        // Calculate how much extra padding in the middle we need between
        // the prompt/message and the indicators on the right
        let prefix_len = prefix_str.len();
        let extra_space = self
            .cols
            .saturating_sub(search_len + prefix_len + prompt_str.len());
        let dsp_prompt: &str = if extra_space == 0 {
            &prompt_str[..self.cols - search_len - prefix_len]
        } else {
            prompt_str
        };

        // push the prompt/msg
        format_string.push_str(dsp_prompt);
        format_string.push_str(&" ".repeat(extra_space));

        // add the prefix_num if it exists
        if prefix_len > 0 {
            format_string.push_str(INPUT_BG);
            format_string.push_str(&prefix_str);
        }

        // and add the search indicator stuff if it exists
        #[cfg(feature = "search")]
        if search_len > 0 {
            format_string.push_str(SEARCH_BG);
            format_string.push_str(&search_str);
        }

        self.displayed_prompt = format_string;
    }

    /// Returns all the text within the bounds, after flattening
    pub(crate) fn get_flattened_lines_with_bounds(&self, start: usize, end: usize) -> &[String] {
        if start >= self.num_lines() || start > end {
            &[]
        } else if end >= self.num_lines() {
            &self.formatted_lines[start..]
        } else {
            &self.formatted_lines[start..end]
        }
    }

    /// Runs the exit callbacks
    pub(crate) fn exit(&mut self) {
        for func in &mut self.exit_callbacks {
            func();
        }
    }

    pub(crate) fn append_str(&mut self, text: &str) {
        let (fmt_line, num_unterminated) = self.make_append_str(text);
        self.append_str_on_unterminated(fmt_line, num_unterminated);
    }

    /// Makes the text that will be displayed and appended it to [`self.formatted_lines`]
    ///
    /// - The first output value is the actual text rows that needs to be appended. This is wrapped
    ///     based on the available columns
    /// - The second value is the number of rows that should be truncated from [`self.formatted_lines`]
    ///     before appending this line. This will be 0 if the given `text` is to be appended to
    ///     [`self.formatted_lines`] but will be `>0` if the given text is actually part of the
    ///     last appended line. This function determines this by checking whether self.lines ends with
    ///     `\n` after appending the text
    pub(crate) fn make_append_str(&mut self, text: &str) -> (Vec<String>, usize) {
        let append = self.lines.ends_with('\n') || self.lines.is_empty();

        let to_format = if append {
            text.to_string()
        } else {
            self.lines.lines().last().unwrap_or("").to_string() + text
        };

        let to_skip = self.lines.lines().count();
        // push the text to lines
        self.lines.push_str(text);
        // And get how many lines of text will be shown (not how many rows, how many wrapped
        // lines), and get its string length
        let line_number = self.lines.lines().count();
        let len_line_number = line_number.to_string().len();
        // This will get filled if there is an ongoing search. We just need to append it to
        // self.search_idx at the end
        #[cfg(feature = "search")]
        let mut append_search_idx = BTreeSet::new();

        // If append is true, we take only the given text for formatting
        // else we also take the last line of self.lines for formatting. This is because we nned to
        // format the entire line rathar than just this part
        let to_format_len = to_format.lines().count();
        let lines = to_format
            .lines()
            .enumerate()
            .map(|(idx, s)| (idx, s.to_string()))
            .collect::<Vec<(usize, String)>>();

        let mut fmtl = Vec::with_capacity(256);

        // First line
        let mut first_line = self.formatted_line(
            // TODO: Remove unwrap from here
            &lines.first().unwrap().1,
            len_line_number,
            to_skip.saturating_sub(1),
            #[cfg(feature = "search")]
            if append {
                self.formatted_lines.len()
            } else {
                self.formatted_lines.len().saturating_sub(1)
            },
            #[cfg(feature = "search")]
            &mut append_search_idx,
        );

        // Format the last line, only if first line and last line are different. We can check this
        // by seeing whether to_format_len is greater than 1
        let last_line = if to_format_len > 1 {
            Some(self.formatted_line(
                &lines.last().unwrap().1,
                len_line_number,
                to_format_len + to_skip.saturating_sub(1),
                #[cfg(feature = "search")]
                self.formatted_lines.len(),
                #[cfg(feature = "search")]
                &mut append_search_idx,
            ))
        } else {
            None
        };

        // Format all other lines except the first and last line
        let mut mid_lines = lines
            .iter()
            .skip(1)
            .take(lines.len().saturating_sub(2))
            .flat_map(|(idx, line)| {
                self.formatted_line(
                    line,
                    len_line_number,
                    idx + to_skip.saturating_sub(1),
                    #[cfg(feature = "search")]
                    self.formatted_lines.len(),
                    #[cfg(feature = "search")]
                    &mut append_search_idx,
                )
            })
            .collect::<Vec<String>>();

        let unterminated = if self.lines.ends_with('\n') {
            0
        } else if to_format_len > 1 {
            last_line.as_ref().unwrap().len()
        } else {
            first_line.len()
        };

        fmtl.append(&mut first_line);
        fmtl.append(&mut mid_lines);
        if let Some(mut ll) = last_line {
            fmtl.append(&mut ll);
        }

        #[cfg(feature = "search")]
        self.search_idx.append(&mut append_search_idx);
        (fmtl, unterminated)
    }

    /// Conditionally appends to [`self.formatted_lines`] or changes the last unterminated rows of
    /// [`self.formatted_lines`]
    ///
    /// `num_unterminated` is the current number of lines returned by [`self.make_append_str`]
    /// that should be truncated from [`self.formatted_lines`] to update the last line
    pub(crate) fn append_str_on_unterminated(
        &mut self,
        mut fmt_line: Vec<String>,
        num_unterminated: usize,
    ) {
        if num_unterminated != 0 || self.unterminated != 0 {
            self.formatted_lines
                .truncate(self.formatted_lines.len() - self.unterminated);
        }
        self.formatted_lines.append(&mut fmt_line);
        self.unterminated = num_unterminated;
    }
}
