//! Contains types that hold run-time information of the pager.

#[cfg(feature = "search")]
use crate::search::{SearchMode, SearchOpts};

use crate::{
    error::{MinusError, TermError},
    input::{self, HashedEventRegister},
    minus_core::{
        self,
        utils::{display::AppendStyle, LinesRowMap},
        CommandQueue,
    },
    screen::{self, Screen},
    ExitStrategy, LineNumbers,
};
use crossterm::{terminal, tty::IsTty};
#[cfg(feature = "search")]
use parking_lot::Condvar;
use parking_lot::Mutex;
#[cfg(feature = "search")]
use std::collections::BTreeSet;
use std::{
    collections::hash_map::RandomState,
    convert::TryInto,
    io::stdout,
    io::Stdout,
    sync::{atomic::AtomicBool, Arc},
};

use crate::minus_core::{commands::Command, ev_handler::handle_event};
use crossbeam_channel::Receiver;

#[cfg(feature = "search")]
#[cfg_attr(docsrs, doc(cfg(feature = "search")))]
#[allow(clippy::module_name_repetitions)]
/// Contains information about the current search
pub struct SearchState {
    /// Direction of search
    ///
    /// See [`SearchMode`] for available options
    pub search_mode: SearchMode,
    /// Stores the most recent search term
    pub(crate) search_term: Option<regex::Regex>,
    /// Lines where searches have a match
    /// In order to avoid duplicate entries of lines, we keep it in a [`BTreeSet`]
    pub(crate) search_idx: BTreeSet<usize>,
    /// Index of search item currently in focus
    /// It should be 0 even when no search is in action
    pub(crate) search_mark: usize,
    /// Function to run before running an incremental search.
    ///
    /// If the function returns a `false`, the incremental search is cancelled.
    pub(crate) incremental_search_condition:
        Box<dyn Fn(&SearchOpts) -> bool + Send + Sync + 'static>,
}

#[cfg(feature = "search")]
impl Default for SearchState {
    fn default() -> Self {
        let incremental_search_condition = Box::new(|so: &SearchOpts| {
            so.string.len() > 1
                && so
                    .incremental_search_options
                    .as_ref()
                    .unwrap()
                    .screen
                    .line_count()
                    <= 5000
        });
        Self {
            search_mode: SearchMode::Unknown,
            search_term: None,
            search_idx: BTreeSet::new(),
            search_mark: 0,
            incremental_search_condition,
        }
    }
}

/// Holds all information and configuration about the pager during
/// its run time.
///
/// This type is exposed so that end-applications can implement the
/// [`InputClassifier`](input::InputClassifier) trait which requires the `PagerState` to be passed
/// as a parameter
///
/// Various fields are made public so that their values can be accessed while implementing the
/// trait.
#[allow(clippy::module_name_repetitions)]
pub struct PagerState {
    /// Configuration for line numbers. See [`LineNumbers`]
    pub line_numbers: LineNumbers,
    /// Any message to display to the user at the prompt
    /// The first element contains the actual message, while the second element tells
    /// whether the message has changed since the last display.
    pub message: Option<String>,
    /// The upper bound of scrolling.
    ///
    /// This is useful for keeping track of the range of lines which are currently being displayed on
    /// the terminal.
    /// When `rows - 1` is added to the `upper_mark`, it gives the lower bound of scroll.
    ///
    /// For example if there are 10 rows is a terminal and the data to display has 50 lines in it
    /// If the `upper_mark` is 15, then the first row of the terminal is the 16th line of the data
    /// and last row is the 24th line of the data.
    pub upper_mark: usize,
    /// The left mark of scrolling
    ///
    /// When this is `> 0`, this amount of text will be truncated from the left side
    pub left_mark: usize,
    /// Direction of search
    ///
    /// See [`SearchMode`] for available options
    ///
    /// **WARNING: This item has been deprecated in favour of [SearchState::search_mode] availlable
    /// by the [PagerState::search_state] field. Any new code should prefer using it instead of this one.**
    #[cfg(feature = "search")]
    #[cfg_attr(docsrs, cfg(feature = "search"))]
    pub search_mode: SearchMode,
    /// Available rows in the terminal
    pub rows: usize,
    /// Available columns in the terminal
    pub cols: usize,
    /// This variable helps in scrolling more than one line at a time
    /// It keeps track of all the numbers that have been entered by the user
    /// until any of `j`, `k`, `G`, `Up` or `Down` is pressed
    pub prefix_num: String,
    /// Describes whether minus is running and in which mode
    pub running: &'static Mutex<crate::RunMode>,
    #[cfg(feature = "search")]
    #[cfg_attr(docsrs, cfg(feature = "search"))]
    pub search_state: SearchState,
    pub screen: Screen,
    /// The prompt displayed at the bottom wrapped to available terminal width
    pub(crate) prompt: String,
    /// The input classifier to be called when a input is detected
    pub(crate) input_classifier: Box<dyn input::InputClassifier + Sync + Send>,
    /// Functions to run when the pager quits
    pub(crate) exit_callbacks: Vec<Box<dyn FnMut() + Send + Sync + 'static>>,
    /// The behaviour to do when user quits the program using `q` or `Ctrl+C`
    /// See [`ExitStrategy`] for available options
    pub(crate) exit_strategy: ExitStrategy,
    /// The prompt that should be displayed to the user, formatted with the
    /// current search index and number of matches (if the search feature is enabled),
    /// and the current numbers inputted to scroll
    pub(crate) displayed_prompt: String,
    /// Whether to show the prompt on the screen
    pub(crate) show_prompt: bool,
    /// Do we want to page if there is no overflow
    #[cfg(feature = "static_output")]
    pub(crate) run_no_overflow: bool,
    pub(crate) lines_to_row_map: LinesRowMap,
    /// Value for follow mode.
    /// See [follow_output](crate::pager::Pager::follow_output) for more info on follow mode.
    pub(crate) follow_output: bool,
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

        let prompt = std::env::current_exe()
            .unwrap_or_else(|_| std::path::PathBuf::from("minus"))
            .file_name()
            .map_or_else(
                || std::ffi::OsString::from("minus"),
                std::ffi::OsStr::to_os_string,
            )
            .into_string()
            .unwrap_or_else(|_| String::from("minus"));

        let mut state = Self {
            line_numbers: LineNumbers::Disabled,
            upper_mark: 0,
            prompt,
            running: &minus_core::RUNMODE,
            left_mark: 0,
            exit_strategy: ExitStrategy::ProcessQuit,
            input_classifier: Box::<HashedEventRegister<RandomState>>::default(),
            exit_callbacks: Vec::with_capacity(5),
            message: None,
            screen: Screen::default(),
            displayed_prompt: String::new(),
            show_prompt: true,
            #[cfg(feature = "static_output")]
            run_no_overflow: false,
            #[cfg(feature = "search")]
            search_mode: SearchMode::default(),
            #[cfg(feature = "search")]
            search_state: SearchState::default(),
            // Just to be safe in tests, keep at 1x1 size
            cols,
            rows,
            prefix_num: String::new(),
            lines_to_row_map: LinesRowMap::new(),
            follow_output: false,
        };

        state.format_prompt();
        Ok(state)
    }

    /// Generate the initial [`PagerState`]
    ///
    /// [`init_core`](crate::minus_core::init::init_core) calls this functions for creating the PagerState.
    ///
    /// This function creates a default [`PagerState`] and fetches all events present in the receiver
    /// to create the initial state. This is done before starting the pager so that
    /// the optimizationss can be applied.
    ///
    /// # Errors
    /// This function will return an error if it could not create the default [`PagerState`] or fails
    /// to process the events
    pub(crate) fn generate_initial_state(
        rx: &Receiver<Command>,
        mut out: &mut Stdout,
    ) -> Result<Self, MinusError> {
        let mut ps = Self::new()?;
        let mut command_queue = CommandQueue::new_zero();
        rx.try_iter().try_for_each(|ev| -> Result<(), MinusError> {
            handle_event(
                ev,
                &mut out,
                &mut ps,
                &mut command_queue,
                &Arc::new(AtomicBool::new(false)),
                #[cfg(feature = "search")]
                &Arc::new((Mutex::new(true), Condvar::new())),
            )
        })?;
        Ok(ps)
    }

    pub(crate) fn format_lines(&mut self) {
        let (buffer, format_result) = screen::make_format_lines(
            &self.screen.orig_text,
            self.line_numbers,
            self.cols,
            self.screen.line_wrapping,
            #[cfg(feature = "search")]
            &self.search_state.search_term,
        );

        #[cfg(feature = "search")]
        {
            self.search_state.search_idx = format_result.append_search_idx;
        }
        self.screen.formatted_lines = buffer;
        self.lines_to_row_map = format_result.lines_to_row_map;
        self.screen.max_line_length = format_result.max_line_length;

        self.screen.unterminated = format_result.num_unterminated;
        self.format_prompt();
    }

    /// Reformat the inputted prompt to how it should be displayed
    pub(crate) fn format_prompt(&mut self) {
        const PROMPT_SPEC: &str = "\x1b[2;40;37m";
        const SEARCH_SPEC: &str = "\x1b[30;44m";
        const INPUT_SPEC: &str = "\x1b[30;43m";
        const MSG_SPEC: &str = "\x1b[30;1;41m";
        const RESET: &str = "\x1b[0m";
        const FOLLOW_MODE_SPEC: &str = "\x1b[1m";

        // Allocate the string. Add extra space in case for the
        // ANSI escape things if we do have characters typed and search showing
        let mut format_string = String::with_capacity(self.cols + (SEARCH_SPEC.len() * 5) + 4);

        // Get the string that will contain the search index/match indicator
        #[cfg(feature = "search")]
        let mut search_str = String::new();
        #[cfg(feature = "search")]
        if !self.search_state.search_idx.is_empty() {
            search_str.push(' ');
            search_str.push_str(&(self.search_state.search_mark + 1).to_string());
            search_str.push('/');
            search_str.push_str(&self.search_state.search_idx.len().to_string());
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

        let follow_mode_str: &str = if self.follow_output { "[F]" } else { "" };

        // Calculate how much extra padding in the middle we need between
        // the prompt/message and the indicators on the right
        let prefix_len = prefix_str.len();
        let extra_space = self
            .cols
            .saturating_sub(search_len + prefix_len + follow_mode_str.len() + prompt_str.len());
        let dsp_prompt: &str = if extra_space == 0 {
            &prompt_str[..self.cols - search_len - prefix_len - follow_mode_str.len()]
        } else {
            prompt_str
        };

        // push the prompt/msg
        if self.message.is_some() {
            format_string.push_str(MSG_SPEC);
        } else {
            format_string.push_str(PROMPT_SPEC);
        }
        format_string.push_str(dsp_prompt);
        format_string.push_str(&" ".repeat(extra_space));

        // add the prefix_num if it exists
        if prefix_len > 0 {
            format_string.push_str(INPUT_SPEC);
            format_string.push_str(&prefix_str);
        }

        // and add the search indicator stuff if it exists
        #[cfg(feature = "search")]
        if search_len > 0 {
            format_string.push_str(SEARCH_SPEC);
            format_string.push_str(&search_str);
        }

        // add follow-mode indicator
        if !follow_mode_str.is_empty() {
            format_string.push_str(FOLLOW_MODE_SPEC);
            format_string.push_str(follow_mode_str);
        }

        format_string.push_str(RESET);

        self.displayed_prompt = format_string;
    }

    /// Runs the exit callbacks
    pub(crate) fn exit(&mut self) {
        for func in &mut self.exit_callbacks {
            func();
        }
    }

    pub(crate) fn append_str(&mut self, text: &str) -> AppendStyle {
        let old_lc = self.screen.line_count();
        let old_lc_dgts = minus_core::utils::digits(old_lc);
        let mut append_result = self.screen.push_screen_buf(
            text,
            self.line_numbers,
            self.cols.try_into().unwrap(),
            #[cfg(feature = "search")]
            &self.search_state.search_term,
        );
        let new_lc = self.screen.line_count();
        let new_lc_dgts = minus_core::utils::digits(new_lc);
        #[cfg(feature = "search")]
        {
            let mut append_search_idx = append_result.append_search_idx;
            self.search_state.search_idx.append(&mut append_search_idx);
        }
        self.lines_to_row_map.append(
            &mut append_result.lines_to_row_map,
            append_result.clean_append,
        );

        if self.line_numbers.is_on() && (new_lc_dgts != old_lc_dgts && old_lc_dgts != 0) {
            self.format_lines();
            return AppendStyle::FullRedraw;
        }

        let total_rows = self.screen.formatted_lines_count();
        let fmt_lines = &self
            .screen
            .get_formatted_lines_with_bounds(total_rows - append_result.rows_formatted, total_rows);
        AppendStyle::PartialUpdate(fmt_lines)
    }
}
