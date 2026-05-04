//! Provides functions for getting analysis of the text data inside minus.
//!
//! This module is still a work is progress and is subject to change.
use crate::{
    LineNumbers,
    minus_core::{self, utils::LinesRowMap},
};
#[cfg(feature = "search")]
use regex::Regex;

use std::{borrow::Cow, fmt};

#[cfg(feature = "search")]
use {crate::search, std::collections::BTreeSet};

// |||||||||||||||||||||||||||||||||||||||||||||||||||||||
//  TYPES TO BETTER DESCRIBE THE PURPOSE OF STRINGS
// |||||||||||||||||||||||||||||||||||||||||||||||||||||||
pub type Row = String;
pub type Rows = Vec<String>;
pub type Line<'a> = &'a str;
pub type TextBlock<'a> = &'a str;
pub type OwnedTextBlock = String;

pub(crate) struct FormattedRow<'a> {
    row: Cow<'a, str>,
    show_line_numbers: bool,
    line_number: Option<usize>,
    padding: usize,
}

impl<'a> FormattedRow<'a> {
    fn raw_row(&self) -> &str {
        &self.row
    }

    fn fmt_prefix(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.show_line_numbers {
            return Ok(());
        }

        match self.line_number {
            Some(line_number) => {
                let line_number = line_number + 1;
                let number_width = minus_core::utils::digits(line_number) + 1;
                let left_padding = self.padding.saturating_sub(number_width);

                write!(f, "{:left_padding$}", "")?;
                if cfg!(not(test)) {
                    write!(f, "{}", crossterm::style::Attribute::Bold)?;
                }
                write!(f, "{line_number}.")?;
                if cfg!(not(test)) {
                    write!(f, "{}", crossterm::style::Attribute::Reset)?;
                }
                f.write_str(" ")
            }
            None => write!(f, "{:>width$} ", "", width = self.padding),
        }
    }
}

impl fmt::Display for FormattedRow<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_prefix(f)?;
        f.write_str(self.raw_row())
    }
}

#[cfg(feature = "search")]
pub(crate) struct SearchFormattedRow<'a, 'b> {
    row: FormattedRow<'a>,
    search_term: Option<&'b Regex>,
    is_match: bool,
}

#[cfg(feature = "search")]
impl fmt::Display for SearchFormattedRow<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.row.fmt_prefix(f)?;

        if self.is_match {
            write!(
                f,
                "{}",
                search::highlight_matches_args(self.row.raw_row(), self.search_term.unwrap(), false)
            )
        } else {
            f.write_str(self.row.raw_row())
        }
    }
}

// ||||||||||||||||||||||||||||||||||||||||||||||
//  SCREEN TYPE AND ITS REKATED FUNCTIONS
// ||||||||||||||||||||||||||||||||||||||||||||||

/// Stores all the data for the terminal
///
/// This can be used by applications to get a basic analysis of the data that minus has captured
/// while formattng it for terminal display.
///
/// Most of the functions of this type are cheap as minus does a lot of caching of the analysis
/// behind the scenes
pub struct Screen {
    pub(crate) orig_text: OwnedTextBlock,
    pub(crate) formatted_lines: Rows,
    pub(crate) line_count: usize,
    pub(crate) max_line_length: usize,
    /// Unterminated lines
    /// Keeps track of the number of lines at the last of [Self::formatted_lines] which are not
    /// terminated by a newline
    pub(crate) unterminated: usize,
    /// Whether to Line wrap lines
    ///
    /// Its negation gives the state of whether horizontal scrolling is allowed.
    pub(crate) line_wrapping: bool,
}

impl Screen {
    /// Get the actual number of physical rows that the text that will actually occupy on the
    /// terminal
    #[must_use]
    pub const fn formatted_lines_count(&self) -> usize {
        self.formatted_lines.len()
    }
    /// Get the number of [`Lines`](std::str::Lines) in the text.
    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.line_count
    }
    /// Returns all the [Rows] within the bounds
    pub(crate) fn get_formatted_lines_with_bounds(&self, start: usize, end: usize) -> &[Row] {
        if start >= self.formatted_lines_count() || start > end {
            &[]
        } else if end >= self.formatted_lines_count() {
            &self.formatted_lines[start..]
        } else {
            &self.formatted_lines[start..end]
        }
    }

    /// Get the length of the longest [Line] in the text.
    #[must_use]
    pub const fn get_max_line_length(&self) -> usize {
        self.max_line_length
    }

    /// Insert the text into the []
    pub(crate) fn push_screen_buf(
        &mut self,
        text: TextBlock,
        line_numbers: LineNumbers,
        cols: u16,
        #[cfg(feature = "search")] search_term: Option<&Regex>,
    ) -> FormatResult {
        // If the last line of self.screen.orig_text is not terminated by than the first line of
        // the incoming text is part of that line so we also need to take care of that.
        //
        // Appropriately in that case we set the last lne of self.screen.orig_text as attachment
        // text for the FormatOpts.
        let clean_append = self.orig_text.ends_with('\n') || self.orig_text.is_empty();
        // We check if number of digits in current line count change during this text push.
        let old_lc = self.line_count();

        // Conditionally appends to [`self.formatted_lines`] or changes the last unterminated rows of
        // [`self.formatted_lines`]
        //
        // `num_unterminated` is the current number of lines returned by [`self.make_append_str`]
        // that should be truncated from [`self.formatted_lines`] to update the last line
        self.formatted_lines
            .truncate(self.formatted_lines.len() - self.unterminated);

        let append_props = {
            let attachment = if clean_append {
                None
            } else {
                self.orig_text.lines().last()
            };

            let formatted_lines_count = self.formatted_lines.len();

            let append_opts = FormatOpts {
                buffer: &mut self.formatted_lines,
                text,
                attachment,
                line_numbers,
                formatted_lines_count,
                lines_count: old_lc,
                prev_unterminated: self.unterminated,
                cols: cols.into(),
                line_wrapping: self.line_wrapping,
                #[cfg(feature = "search")]
                search_term,
            };
            format_text_block(append_opts)
        };
        self.orig_text.push_str(text);

        let (num_unterminated, lines_formatted, max_line_length) = (
            append_props.num_unterminated,
            append_props.lines_formatted,
            append_props.max_line_length,
        );

        self.line_count = old_lc + lines_formatted.saturating_sub(usize::from(!clean_append));
        if max_line_length > self.max_line_length {
            self.max_line_length = max_line_length;
        }

        self.unterminated = num_unterminated;
        append_props
    }
}

impl Default for Screen {
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

// |||||||||||||||||||||||||||||||
// TEXT FORMATTING FUNCTIONS
// |||||||||||||||||||||||||||||||

// minus has a very interesting but simple text model that you must go through to understand how minus works.
//
// # Text Block
// A text block in minus is just a bunch of text that may contain newlines (`\n`) between them.
// [`PagerState::lines`] is nothing but just a giant text block.
//
// # Line
// A line is text that must not contain any newlines inside it but may or may not end with a newline.
// Don't confuse this with Rust's [Lines](std::str::Lines) which is similar to minus's Lines terminolagy but only
// differs for the fact that they don't end with a newline. Although the Rust's Lines is heavily used inside minus
// as an important building block.
//
// # Row
// A row is part of a line that fits perfectly inside one row of terminal. Out of the three text types, only row
// is dependent on the terminal conditions. If the terminal gets resized, each row will grow or shrink to hold
// more or less text inside it.
//
// # Termination
// # Termination of Line
// A line is called terminated when it ends with a newline character, otherwise it is called unterminated.
// You may ask why is this important? Because minus supports completing a line in multiple steps, if we don't care
// whether a line is terminated or not, we won't know that the data coming right now is part of the current line or
// it is for a new line.
//
// # Termination of block
// A block is terminated if the last line of the block is terminated i.e it ends with a newline character.
//
// # Unterminated rows
// It is 0 in most of the cases. The only case when it has a non-zero value is a line or block of text is unterminated
// In this case, it is equal to the number of rows that the last line of the block or a the line occupied.
//
// Whenever new data comes while a line or block is unterminated minus cleans up the number of unterminated rows
// on the terminal i.e the entire last line. Then it merges the incoming data to the last line and then reprints
// them on the terminal.
//
// Why this complex approach?
// Simple! printing an entire page on the terminal is slow and this approach allows minus to reprint only the
// parts that are required without having to redraw everything
//
// [`PagerState::lines`]: crate::state::PagerState::lines

pub(crate) trait AppendableBuffer {
    fn push_row(&mut self, row: Row);
}

impl AppendableBuffer for Rows {
    fn push_row(&mut self, row: Row) {
        self.push(row);
    }
}

impl AppendableBuffer for &mut Rows {
    fn push_row(&mut self, row: Row) {
        self.push(row);
    }
}

pub(crate) struct FormatOpts<'a, B>
where
    B: AppendableBuffer,
{
    /// Buffer to insert the text into
    pub buffer: B,
    /// Contains the incoming text data
    pub text: TextBlock<'a>,
    /// This is Some when the last line inside minus's present data is unterminated. It contains the
    /// last line to be attached to the the incoming text
    pub attachment: Option<TextBlock<'a>>,
    /// Status of line numbers
    pub line_numbers: LineNumbers,
    /// This is equal to the number of lines in [`Screen::orig_text`]. This basically tells what
    /// line number the upcoming line will hold.
    pub lines_count: usize,
    /// This is equal to the number of lines in [`Screen::formatted_lines`]. This is used to
    /// calculate the search index of the rows of the line.
    pub formatted_lines_count: usize,
    /// Actual number of columns available for displaying
    pub cols: usize,
    /// Number of lines that are previously unterminated. It is only relevant when there is
    /// `attachment` text otherwise it should be 0.
    pub prev_unterminated: usize,
    /// Search term if a search is active
    #[cfg(feature = "search")]
    pub search_term: Option<&'a regex::Regex>,

    /// Value of [`Screen::line_wrapping`]
    pub line_wrapping: bool,
}

/// Contains the formatted rows along with some basic information about the text formatted
///
/// The basic information includes things like the number of lines formatted or the length of
/// longest line encountered. These are tracked as each line is being formatted hence we refer to
/// them as **tracking variables**.
#[derive(Debug)]
pub(crate) struct FormatResult {
    // **Tracking variables**
    //
    /// Number of lines that have been formatted from `text`.
    pub lines_formatted: usize,
    /// Number of rows that have been formatted from `text`.
    pub rows_formatted: usize,
    /// Number of rows that are unterminated
    pub num_unterminated: usize,
    /// If search is active, this contains the indices where search matches in the incoming text have been found
    #[cfg(feature = "search")]
    pub append_search_idx: BTreeSet<usize>,
    /// Map of where first row of each line is placed inside in [`Screen::formatted_lines`]
    pub lines_to_row_map: LinesRowMap,
    /// The length of longest line encountered in the formatted text block
    pub max_line_length: usize,
    pub clean_append: bool,
}

/// Makes the text that will be displayed.
#[allow(clippy::too_many_lines)]
pub(crate) fn format_text_block<B>(mut opts: FormatOpts<'_, B>) -> FormatResult
where
    B: AppendableBuffer,
{
    // Formatting a text block not only requires us to format each line according to the terminal
    // configuration and the main applications's preference but also gather some basic information
    // about the text that we formatted. The basic information that we gather is supplied along
    // with the formatted lines in the FormatResult's tracking variables.
    //
    // This is a high level overview of how the text formatting works.
    //
    // For a text block, we hae a couple of things to care about:-
    // * Each line is formatted using the using the `formatted_line()` function.
    //   After a line has been formatted using the `formatted_line()` function, calling `.len()` on
    //   the returned vector will give the number of rows that it would span on the terminal.
    //   For less confusion, we call this *row span of that line*.
    // * The first line can have an attachment, in the sense that it can be part of the last line of the
    //   already present text. In that case the FrmatResult::attachment will hold a `Some(...)`
    //   value. `clean_append` keeps track of this: it will be false if an attachment is available.
    // * Formatting of the lines between the first line and last line ie. *middle lines*, is actually
    //   rather simple: we simply format them
    // * The last is also similar to the middle lines except for one exception:-
    //
    //      If it isn't terminated by a \n then we need to find how many rows it
    //      will span in the terminal and set it to the `unterminated` count.
    //
    //   More on this is described in the unterminated section.
    //
    // * We also have more things to take care like `append_search_idx` but most of these
    //   either documented in their respective section or self-understanable so not discussed here.
    //
    // Now the good stuff...
    // * First, if there's an attachment, we merge it with the actual text to be formatted
    //   and tweak certain parameters (see below)
    // * Then  we split the entire text block into two parts: rest_lines and last_line.
    // * Next we format the rest_lines, and all update the tracking variables.
    // * Next we format the last line and keep it separate to calculate unterminated.
    // * If there's exactly one line to format, it will automatically behave as last_line and there
    //   will be no rest_lines.
    // * After all the formatting is done, we return the format results.

    // Compute the text to be format and set clean_append
    let to_format = if let Some(attached_text) = opts.attachment {
        // Tweak certain parameters if we are joining the last line of already present text with the first line of
        // incoming text.
        //
        // First reduce line count by 1 if, because the first line of the incoming text should have the same line
        // number as the last line. Hence all subsequent lines must get a line number less than expected.
        //
        // Next subtract the number of rows that the last line occupied from formatted_lines_count since it is
        // also getting reformatted. This can be easily accomplished by taking help of [`PagerState::unterminated`]
        // which we get in opts.prev_unterminated.
        opts.lines_count = opts.lines_count.saturating_sub(1);
        opts.formatted_lines_count = opts
            .formatted_lines_count
            .saturating_sub(opts.prev_unterminated);
        let mut s = String::with_capacity(opts.text.len() + attached_text.len());
        s.push_str(attached_text);
        s.push_str(opts.text);

        s
    } else {
        opts.text.to_string()
    };

    let lines = to_format
        .lines()
        .enumerate()
        .collect::<Vec<(usize, &str)>>();

    let to_format_size = lines.len();

    let mut fr = FormatResult {
        lines_formatted: to_format_size,
        rows_formatted: 0,
        num_unterminated: opts.prev_unterminated,
        #[cfg(feature = "search")]
        append_search_idx: BTreeSet::new(),
        lines_to_row_map: LinesRowMap::new(),
        max_line_length: 0,
        clean_append: opts.attachment.is_none(),
    };

    let line_number_digits = minus_core::utils::digits(opts.lines_count + to_format_size);

    // Return if we have nothing to format
    if lines.is_empty() {
        return fr;
    }

    // Number of rows that have been formatted so far
    // Whenever a line is formatted, this will be incremented to te number of rows that the formatted line has occupied
    let mut formatted_row_count = opts.formatted_lines_count;

    let (last_idx, last_line_text) = lines.last().copied().unwrap();
    for (idx, line) in lines.iter().take(lines.len().saturating_sub(1)) {
        fr.lines_to_row_map.insert(formatted_row_count, true);
        fr.max_line_length = fr.max_line_length.max(line.len());

        let rows = format_line(
            line,
            line_number_digits,
            opts.lines_count + idx,
            opts.line_numbers,
            opts.cols,
            opts.line_wrapping,
        );

        #[cfg(feature = "search")]
        let rows = format_search_rows(rows, opts.search_term);

        #[cfg(feature = "search")]
        {
            formatted_row_count += collect_rows(
                &mut opts.buffer,
                rows,
                formatted_row_count,
                &mut fr.append_search_idx,
            );
        }

        #[cfg(not(feature = "search"))]
        {
            formatted_row_count += collect_rows(&mut opts.buffer, rows);
        }
    }

    let last_line = format_line(
        last_line_text,
        line_number_digits,
        opts.lines_count + last_idx,
        opts.line_numbers,
        opts.cols,
        opts.line_wrapping,
    );
    #[cfg(feature = "search")]
    let last_line = format_search_rows(last_line, opts.search_term);

    let last_line_rows = last_line.size_hint().1.unwrap();

    fr.lines_to_row_map.insert(formatted_row_count, true);
    fr.max_line_length = fr.max_line_length.max(last_line_text.len());

    #[cfg(feature = "search")]
    {
        formatted_row_count += collect_rows(
            &mut opts.buffer,
            last_line,
            formatted_row_count,
            &mut fr.append_search_idx,
        );
    }

    #[cfg(not(feature = "search"))]
    {
        formatted_row_count += collect_rows(&mut opts.buffer, last_line);
    }

    // Calculate number of rows which are part of last line and are left unterminated  due to absence of \n
    fr.num_unterminated = if opts.text.ends_with('\n') {
        // If the last line ends with \n, then the line is complete so nothing is left as unterminated
        0
    } else {
        last_line_rows
    };
    fr.rows_formatted = formatted_row_count - opts.formatted_lines_count;

    fr
}

pub(crate) fn format_line<'a>(
    line: Line<'a>,
    len_line_number: usize,
    line_number: usize,
    show_line_numbers: LineNumbers,
    cols: usize,
    line_wrapping: bool,
) -> impl Iterator<Item = FormattedRow<'a>> {
    assert!(
        !line.contains('\n'),
        "Newlines found in appending line {:?}",
        line
    );
    let line_numbers = matches!(
        show_line_numbers,
        LineNumbers::Enabled | LineNumbers::AlwaysOn
    );

    // NOTE: Only relevant when line numbers are active
    // Padding is the space that the actual line text will be shifted to accommodate for
    // line numbers. This is equal to:-
    // LineNumbers::EXTRA_PADDING + len_line_number + 1 (for '.') + 1 (for 1 space)
    //
    // We reduce this from the number of available columns as this space cannot be used for
    // actual line display when wrapping the lines
    let padding = len_line_number + LineNumbers::EXTRA_PADDING + 1;

    let cols_avail = if line_numbers {
        cols.saturating_sub(padding + 2)
    } else {
        cols
    };

    // Wrap the line and return an iterator over all the rows
    let enumerated_rows = if line_wrapping {
        textwrap::wrap(line, cols_avail)
    } else {
        vec![Cow::from(line)]
    }
    .into_iter()
    .enumerate();

    enumerated_rows.map(move |(i, row)| {
        FormattedRow {
            row,
            show_line_numbers: line_numbers,
            line_number: if line_numbers && i == 0 {
                Some(line_number)
            } else {
                None
            },
            padding,
        }
    })
}

#[cfg(feature = "search")]
pub(crate) fn format_search_rows<'a>(
    rows: impl Iterator<Item = FormattedRow<'a>> + 'a,
    search_term: Option<&'a Regex>,
) -> impl Iterator<Item = (SearchFormattedRow<'a, 'a>, bool)> + 'a {
    rows.map(move |row| {
        let is_match = search_term.is_some_and(|st| st.is_match(row.raw_row()));
        (
            SearchFormattedRow {
                row,
                search_term,
                is_match,
            },
            is_match,
        )
    })
}

#[cfg(feature = "search")]
fn collect_rows<B, I, D>(
    buffer: &mut B,
    rows: I,
    formatted_idx: usize,
    search_idx: &mut BTreeSet<usize>,
) -> usize
where
    B: AppendableBuffer,
    I: IntoIterator<Item = (D, bool)>,
    D: fmt::Display,
{
    let mut row_count = 0;
    for (wrap_idx, (row, is_match)) in rows.into_iter().enumerate() {
        if is_match {
            search_idx.insert(formatted_idx + wrap_idx);
        }
        buffer.push_row(row.to_string());
        row_count = wrap_idx + 1;
    }
    row_count
}

#[cfg(not(feature = "search"))]
fn collect_rows<B, I, D>(buffer: &mut B, rows: I) -> usize
where
    B: AppendableBuffer,
    I: IntoIterator<Item = D>,
    D: fmt::Display,
{
    let mut row_count = 0;
    for row in rows {
        buffer.push_row(row.to_string());
        row_count += 1;
    }
    row_count
}

/// Formats the given `line`
///
/// - `line`: The line to format
/// - `line_numbers`: tells whether to format the line with line numbers.
/// - `len_line_number`: is the number of digits that number of lines in [`Screen::orig_text`]
///   occupy. For example, this will be 2 if number of lines in [`Screen::line_count`] is 50 and 3
///   if the number of lines in [`Screen::line_count`] is 500. This is used for calculating the
///   padding of each displayed line.
/// - `idx`: is the position index where the line is placed in [`Screen::orig_text`].
/// - `formatted_idx`: is the position index where the line will be placed in the resulting
///   [`Screen::formatted_lines`]
/// - `cols`: Number of columns in the terminal
/// - `search_term`: Contains the regex if a search is active
#[allow(clippy::too_many_arguments)]
#[allow(clippy::uninlined_format_args)]
pub(crate) fn formatted_line<'a>(
    line: Line<'a>,
    len_line_number: usize,
    idx: usize,
    line_numbers: LineNumbers,
    cols: usize,
    line_wrapping: bool,
    #[cfg(feature = "search")] formatted_idx: usize,
    #[cfg(feature = "search")] search_idx: &mut BTreeSet<usize>,
    #[cfg(feature = "search")] search_term: Option<&regex::Regex>,
) -> Rows {
    let rows = format_line(
        line,
        len_line_number,
        idx,
        line_numbers,
        cols,
        line_wrapping,
    );
    let mut formatted_rows = Vec::with_capacity(256);

    #[cfg(feature = "search")]
    {
        let rows = format_search_rows(rows, search_term);
        collect_rows(&mut formatted_rows, rows, formatted_idx, search_idx);
    }

    #[cfg(not(feature = "search"))]
    {
        collect_rows(&mut formatted_rows, rows);
    }

    formatted_rows
}

pub(crate) fn make_format_lines(
    text: &String,
    line_numbers: LineNumbers,
    cols: usize,
    line_wrapping: bool,
    #[cfg(feature = "search")] search_term: Option<&regex::Regex>,
) -> (Rows, FormatResult) {
    let mut buffer = Vec::with_capacity(256);
    let format_opts = FormatOpts {
        buffer: &mut buffer,
        text,
        attachment: None,
        line_numbers,
        formatted_lines_count: 0,
        lines_count: 0,
        prev_unterminated: 0,
        cols,
        #[cfg(feature = "search")]
        search_term,
        line_wrapping,
    };
    let fr = format_text_block(format_opts);
    (buffer, fr)
}

#[cfg(test)]
mod tests;
