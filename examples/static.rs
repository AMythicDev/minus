use std::fmt::Write;

fn main() -> minus::Result {
    let mut output = String::new();
    for i in 1..=30 {
        let _ = writeln!(output, "{}", i);
    }
    minus::page_all(&output)
}
