/// Position. テキスト上の位置
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct APos {
    /// 行番号 (0 から始まる)
    pub(crate) row: usize,
    /// 列番号 (0 から始まる、UTF-8 基準、バイト単位)
    pub(crate) column: usize,
}

impl APos {
    pub(crate) fn from_str(s: &str) -> Self {
        let mut pos = APos::default();

        for c in s.chars() {
            if c == '\n' {
                pos.row += 1;
                pos.column = 0;
            } else {
                pos.column += c.len_utf8();
            }
        }

        pos
    }

    pub(crate) fn add(mut self, other: Self) -> Self {
        if other.row >= 1 {
            self.column = 0;
        }

        self.row += other.row;
        self.column += other.column;
        self
    }
}
