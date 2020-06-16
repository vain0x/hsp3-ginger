use super::APos;

/// テキスト上の範囲
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ARange {
    pub(crate) start: APos,
    pub(crate) end: APos,
}
