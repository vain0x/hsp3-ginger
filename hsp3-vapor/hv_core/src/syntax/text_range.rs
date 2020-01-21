use super::*;

/// テキスト上の範囲を行番号・列番号で指定したもの。
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TextRange {
    start: TextPosition,
    end: TextPosition,
}

impl TextRange {
    pub(crate) fn new(start: TextPosition, end: TextPosition) -> Self {
        TextRange { start, end }
    }

    pub(crate) fn start(&self) -> TextPosition {
        self.start
    }

    pub(crate) fn end(&self) -> TextPosition {
        self.end
    }
}
