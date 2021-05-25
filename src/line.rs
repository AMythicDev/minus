// #![allow(dead_code)]
use std::fmt::Formatter;

#[derive(Clone, Debug)]
/// A wrapper for a collection of Lines
pub(crate) struct WrappedLines(Vec<Line>);

impl WrappedLines {
    /// Create a new WrappedLines instance with an empty Vec
    pub(crate) fn new() -> WrappedLines {
        Self(Vec::new())
    }
    /// Get the total number of rows that are perfectly single row guaranteed
    ///
    /// See [Single Row Guarantee](struct.Line.html#single-row-guarantee)  for more info on it
    pub(crate) fn get_screen_count(&self) -> usize {
        let mut total = 0;
        for l in &self.0 {
            total += l.rows.len();
        }
        total
    }
    /// Apped a new [`Line`]
    pub(crate) fn push(&mut self, line: Line) {
        self.0.push(line);
    }
    /// Return a iterator that allows modifying each value
    pub(crate) fn iter_mut(&mut self) -> std::slice::IterMut<'_, Line> {
        self.0.iter_mut()
    }
    /// An ailas for [`WrappedLines::get_screen_count`]
    pub(crate) fn len(&self) -> usize {
        self.get_screen_count()
    }

    /// Returns a iterator for all the rows that are single row guaranteed
    pub(crate) fn iter_screen(&self) -> std::vec::IntoIter<String> {
        let mut screen_lines = Vec::new();

        for line in &self.0 {
            screen_lines.extend_from_slice(&line.rows);
        }
        screen_lines.into_iter()
    }

    /// Returns a iterator that contains a formatted string containg the line and
    /// it's line number
    pub(crate) fn line_no_annotated(
        &mut self,
        cols: usize,
        len_line_number: usize,
    ) -> std::vec::IntoIter<String> {
        for line in self.iter_mut() {
            // 2 for extra padding
            line.readjust_line(cols - len_line_number - 2);
            let line_no = line.line_no;
            for term_line in &mut line.rows {
                term_line.insert_str(
                    0,
                    &format!("{number: >len$}. ", number = line_no, len = len_line_number,),
                );
            }
        }
        self.iter_screen()
    }
}

impl From<Vec<Line>> for WrappedLines {
    fn from(t: Vec<Line>) -> Self {
        Self(t)
    }
}

impl From<std::slice::IterMut<'_, Line>> for WrappedLines {
    fn from(t: std::slice::IterMut<'_, Line>) -> Self {
        let mut lines = Vec::with_capacity(t.len());
        t.for_each(|l| lines.push(l.clone()));
        Self::from(lines)
    }
}

/// A struct representing a single line
///
/// The struct holds a vector and line number of itself in reference to a bigger text body
/// The vector contains the actual lines that are guaranteed to occupy a single row in the terminal
///
/// # Single Row Guarantee
/// A single row guarantee is available when the rows in a `Line` exactly take up one row
/// in the terminal.  
/// If a `Line` is if created with a line of text which requires two rows in the terminal, then the
/// `Line` struct will internally break it into a `Vec<String>` of two elements, the first element
/// contains the the line that can be fitted into one row and the next element contains whatever
/// is left off.
#[derive(Debug, Clone)]
pub(crate) struct Line {
    /// The line number
    line_no: usize,
    /// The contents of line broken on the amount of columns available in the terminal
    rows: Vec<String>,
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.rows.join(""))?;
        Ok(())
    }
}

impl Line {
    /// Makes a new Line instance where it's contents are single row guaranteed
    pub(crate) fn new(line: impl Into<String>, line_no: usize, cols: usize) -> Self {
        let term_lines = Self::break_line(&line.into(), cols);

        Self {
            line_no,
            rows: term_lines,
        }
    }
    /// Returns the number of breaks that are required to make the `Line` single row safe
    pub(crate) fn calc_breaks(line: &str, cols: usize) -> usize {
        (line.len() / cols).saturating_add(1)
    }

    /// Break the line into single row guaranteed lines
    pub(crate) fn break_line(mut line: &str, cols: usize) -> Vec<String> {
        let breaks = Self::calc_breaks(line, cols);
        // let mut term_lines = Vec::with_capacity(breaks);
        // for _ in 1..breaks {
        //     let (line_1, line_2) = line.split_at(cols);
        //     term_lines.push(line_1.to_owned());
        //     line = line_2;
        // }
        // term_lines.push(line.to_string());
        // term_lines
    }

    #[allow(dead_code)]
    // TODO: Remove this in future and directly call .len() in tests
    pub(crate) fn get_wrapped_count(&self) -> usize {
        self.rows.len()
    }

    /// Readjust the breaks for a new `cols` parameter
    pub(crate) fn readjust_line(&mut self, cols: usize) {
        let line = self.to_string();
        self.rows = Self::break_line(&line, cols);
    }

    /// Returns a vec of `Line` for a string of text with unknown number of
    /// lines
    pub(crate) fn from_str(text: &str, cols: usize) -> Vec<Line> {
        let mut lines = Vec::new();
        for (idx, line) in text.lines().enumerate() {
            lines.push(Self::new(line, idx + 1, cols));
        }
        lines
    }

    /// Returns a iterator for all the rows
    pub(crate) fn iter_mut(&mut self) -> std::slice::IterMut<String> {
        self.rows.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::Line;
    const COLS: usize = 80;

    #[test]
    fn test_line_new() {
        let mut test_str = String::new();

        for _ in 0..200 {
            test_str.push('#')
        }
        let result = Line::new(test_str, 0, COLS).rows;
        assert_eq!(200 / COLS + 1, result.len());
        assert_eq!(
            (COLS, COLS, 200 - COLS * 2),
            (result[0].len(), result[1].len(), result[2].len())
        );
    }
    #[test]
    fn test_line_from_str() {
        let mut test_str = String::new();
        for _ in 0..10 {
            for _ in 0..200 {
                test_str.push('#')
            }
            test_str.push('\n')
        }
        let result = Line::from_str(&test_str, COLS);
        let mut total = 0;
        for tl in result {
            assert_eq!(3, tl.get_wrapped_count());
            total += tl.get_wrapped_count();
        }
        assert_eq!((200 / COLS + 1) * 10, total);
    }
}
