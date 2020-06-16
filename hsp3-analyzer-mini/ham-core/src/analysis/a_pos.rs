/// Position. テキスト上の位置
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct APos {
    /// 行番号 (0 から始まる)
    pub(crate) row: usize,
    /// 列番号 (0 から始まる、UTF-8 基準、バイト単位)
    pub(crate) column: usize,
}
