use arrayvec::ArrayString;

type InsertionBuffer = ArrayString<240>;

pub trait DataSource {
    fn reads_forward(&mut self, buffer: &mut InsertionBuffer) -> bool;
    fn reads_backward(&mut self, buffer: &mut InsertionBuffer) -> bool;
    fn readr_line(&mut self, line: &mut String, pb: &mut InsertionBuffer, fb: &mut InsertionBuffer);
}

pub struct StringSource {
    text: String,
    cursor: usize,
}

impl DataSource for StringSource {
    fn reads_forward(&mut self, buffer: &mut InsertionBuffer) -> bool {
        let max_read = (self.text.len() - self.cursor).min(buffer.capacity());
        buffer.push_str(&self.text[self.cursor..self.cursor + max_read]);
        self.cursor += max_read + 1;

        self.cursor < self.text.len()
    }
    fn reads_backward(&mut self, buffer: &mut InsertionBuffer) -> bool {
        let max_read = self.text.len().min(buffer.capacity());
        buffer.push_str(&self.text[self.cursor..max_read]);
        self.cursor += max_read + 1;

        self.cursor != 0
    }

    fn readr_line(
        &mut self,
        line: &mut String,
        pb: &mut InsertionBuffer,
        fb: &mut InsertionBuffer,
    ) {
    }
}
