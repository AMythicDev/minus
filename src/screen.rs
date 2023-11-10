use crossterm::{terminal, tty::IsTty};
use std::{collections::BTreeSet, io::stdout};

use crate::{
    minus_core::{screen_line::ScreenLine, utils::text::format_prompt},
    MinusError, Result,
};

pub struct Screen {
    screen_lines: Vec<ScreenLine>,
    upper_mark: usize,
    rows: usize,
    cols: usize,
    displayed_prompt: String,
    prompt: String,
}

impl Screen {
    pub(crate) fn new() -> Result<Self, MinusError> {
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

        Ok(Self {
            screen_lines: Vec::with_capacity(1024),
            rows,
            cols,
            upper_mark: 1,
            prompt: String::new(),
            displayed_prompt: String::new(),
        })
    }

    pub(crate) fn new_with_prompt(prompt: String) -> Result<Self, MinusError> {
        let mut screen = Self::new()?;
        screen.prompt = prompt;
        screen.displayed_prompt = format_prompt(
            prompt,
            screen.cols,
            "",
            None,
            #[cfg(feature = "search")]
            BTreeSet::new(),
            #[cfg(feature = "search")]
            0,
        );
        Ok(screen)
    }
}
