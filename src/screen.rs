//! Provides functions for getting analysis of the text data inside minus.
//!
//! This module is still a work is progress and is subject to change.
pub struct Screen {
    pub(crate) orig_text: String,
    pub(crate) formatted_lines: Vec<String>,
    pub(crate) line_count: usize,
}

impl Screen {
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
    pub(crate) fn get_formatted_lines_with_bounds(&self, start: usize, end: usize) -> &[String] {
        if start >= self.formatted_lines_count() || start > end {
            &[]
        } else if end >= self.formatted_lines_count() {
            &self.formatted_lines[start..]
        } else {
            &self.formatted_lines[start..end]
        }
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            orig_text: String::with_capacity(100 * 1024),
            formatted_lines: Vec::with_capacity(500 * 1024),
            line_count: 0,
        }
    }
}
