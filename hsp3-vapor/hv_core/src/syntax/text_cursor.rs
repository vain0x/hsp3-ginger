use super::*;

/// テキスト位置を計算するもの。
#[derive(Clone, Default)]
pub(crate) struct TextCursor {
    /// 0-based index.
    line: usize,

    /// 0-based index. (UTF-16 基準)
    character: usize,
}

impl TextCursor {
    pub(crate) fn current(&self) -> TextPosition {
        TextPosition::new(self.line + 1, self.character + 1)
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
