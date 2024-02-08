pub mod display;
pub mod term;
pub mod text;

/// Return the number of digits in `num`
pub const fn digits(num: usize) -> usize {
    (if num == 0 { 0 } else { num.ilog10() as usize }) + 1
}
