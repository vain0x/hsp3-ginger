/// 行番号と列番号で表されるテキスト上の位置。(1 から始まる。)
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TextPosition {
    /// 1-based index.
    line: usize,

    /// 1-based index.
    character: usize,
}

impl TextPosition {
    pub(crate) fn new(line: usize, character: usize) -> Self {
        assert!(line >= 1);
        assert!(character >= 1);

        TextPosition { line, character }
    }

    pub(crate) fn line(&self) -> usize {
        self.line
    }

    pub(crate) fn character(&self) -> usize {
        self.character
    }
}
