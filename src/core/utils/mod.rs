pub mod display;
pub mod term;

/// Return the number of digits in `num`
pub const fn digits(num: usize) -> usize {
    (if num == 0 { 0 } else { num.ilog10() as usize }) + 1
}

/// Stores the location of first row each line
///
/// Due to line wrapping, each line may or may not occupy exactly one row on the terminal
/// Hence we nned to keep track where the first row o each line is positioned in the entire text
/// array.
#[derive(Debug)]
pub struct LinesRowMap(Vec<usize>);

impl LinesRowMap {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn append(&mut self, idx: &mut Self, clean_append: bool) {
        if !clean_append {
            self.0.pop();
        }
        self.0.append(&mut idx.0);
    }

    pub fn insert(&mut self, ln: usize, clean_append: bool) {
        if !clean_append {
            self.0.pop();
        }
        self.0.push(ln);
    }

    pub fn get(&self, ln: usize) -> Option<&usize> {
        self.0.get(ln)
    }
}
