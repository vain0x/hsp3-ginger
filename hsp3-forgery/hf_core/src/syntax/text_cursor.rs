use super::*;

#[derive(Clone, Default)]
pub(crate) struct TextCursor {
    /// 0-based index.
    line: usize,

    /// 0-based index.
    character: usize,
}

impl TextCursor {
    pub(crate) fn current(&self) -> Position {
        Position {
            line: self.line,
            character: self.character,
        }
    }

    pub(crate) fn advance(&mut self, text: &str) {
        for c in text.chars() {
            if c == '\n' {
                self.line += 1;
                self.character = 0;
            } else {
                self.character += c.len_utf16();
            }
        }
    }
}
