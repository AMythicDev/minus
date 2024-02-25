//! Provides functions for getting analysis of the text data inside minus.
//!
//! This module is still a work is progress and is subject to change.

use crate::{
    minus_core::{
        self,
        utils::text::{self, AppendStyle},
    },
    LineNumbers,
};
#[cfg(feature = "search")]
use regex::Regex;

type Row = String;
type Rows = Vec<String>;
type Line = String;
type TextBlock<'a> = &'a str;
type OwnedTextBlock = String;

pub struct ScreenData {
    pub(crate) orig_text: OwnedTextBlock,
    pub(crate) formatted_lines: Rows,
    pub(crate) line_count: usize,
    pub(crate) max_line_length: usize,
    /// Unterminated lines
    /// Keeps track of the number of lines at the last of [PagerState::formatted_lines] which are
    /// not terminated by a newline
    pub(crate) unterminated: usize,
    /// Whether to Line wrap lines
    ///
    /// Its negation gives the state of whether horizontal scrolling is allowed.
    pub(crate) line_wrapping: bool,
}

impl ScreenData {
    /// Get the actual number of physical rows from the text that will be printed on the terminal
    #[must_use]
    pub fn formatted_lines_count(&self) -> usize {
        self.formatted_lines.len()
    }
    /// Get the number of [`Lines`](std::str::Lines) in the text.
    ///
    /// NOTE: This operation might be expensive if the text data is too large.
    #[must_use]
    pub const fn get_line_count(&self) -> usize {
        self.line_count
    }
    /// Returns all the text within the bounds
    pub(crate) fn get_formatted_lines_with_bounds(&self, start: usize, end: usize) -> &[Row] {
        if start >= self.formatted_lines_count() || start > end {
            &[]
        } else if end >= self.formatted_lines_count() {
            &self.formatted_lines[start..]
        } else {
            &self.formatted_lines[start..end]
        }
    }

    #[must_use]
    pub const fn get_max_line_length(&self) -> usize {
        self.max_line_length
    }

    pub(crate) fn append_str(
        &mut self,
        text: &str,
        line_numbers: LineNumbers,
        cols: u16,
        #[cfg(feature = "search")] search_term: &Option<Regex>,
    ) -> AppendStyle {
        // If the last line of self.screen.orig_text is not terminated by than the first line of
        // the incoming text is part of that line so we also need to take care of that.
        //
        // Appropriately in that case we set the last lne of self.screen.orig_text as attachment
        // text for the FormatOpts.
        let clean_append = self.orig_text.ends_with('\n') || self.orig_text.is_empty();
        let attachment = if clean_append {
            None
        } else {
            self.orig_text.lines().last().map(ToString::to_string)
        };

        // We check if number of digits in current line count change during this text push.
        let old_lc = self.get_line_count();
        let old_lc_dgts = minus_core::utils::digits(old_lc);

        self.orig_text.push_str(text);

        let append_opts = text::FormatOpts {
            text,
            attachment,
            line_numbers,
            formatted_lines_count: self.formatted_lines.len(),
            lines_count: old_lc,
            prev_unterminated: self.unterminated,
            cols: cols.into(),
            line_wrapping: self.line_wrapping,
            #[cfg(feature = "search")]
            search_term,
        };

        let append_props = text::format_text_block(append_opts);

        let (
            mut fmt_line,
            num_unterminated,
            mut lines_to_row_map,
            lines_formatted,
            max_line_length,
        ) = (
            append_props.text,
            append_props.num_unterminated,
            append_props.lines_to_row_map,
            append_props.lines_formatted,
            append_props.max_line_length,
        );

        let new_lc = old_lc + lines_formatted.saturating_sub(usize::from(!clean_append));
        self.line_count = new_lc;
        self.max_line_length = max_line_length;
        let new_lc_dgts = minus_core::utils::digits(new_lc);

        #[cfg(feature = "search")]
        {
            let mut append_search_idx = append_props.append_search_idx;
            self.search_state.search_idx.append(&mut append_search_idx);
        }
        self.lines_to_row_map
            .append(&mut lines_to_row_map, clean_append);

        if self.line_numbers.is_on() && (new_lc_dgts != old_lc_dgts && old_lc_dgts != 0) {
            self.format_lines();
            return AppendStyle::FullRedraw;
        }

        // Conditionally appends to [`self.formatted_lines`] or changes the last unterminated rows of
        // [`self.formatted_lines`]
        //
        // `num_unterminated` is the current number of lines returned by [`self.make_append_str`]
        // that should be truncated from [`self.formatted_lines`] to update the last line
        self.formatted_lines
            .truncate(self.formatted_lines.len() - self.unterminated);
        self.unterminated = num_unterminated;
        if self.running.lock().is_uninitialized() {
            self.formatted_lines.append(&mut fmt_line);
            return AppendStyle::NoDraw;
        }

        self.formatted_lines.append(&mut fmt_line.clone());

        AppendStyle::PartialUpdate(fmt_line)
    }
}

impl Default for ScreenData {
    fn default() -> Self {
        Self {
            line_wrapping: true,
            orig_text: String::with_capacity(100 * 1024),
            formatted_lines: Vec::with_capacity(500 * 1024),
            line_count: 0,
            max_line_length: 0,
            unterminated: 0,
        }
    }
}
