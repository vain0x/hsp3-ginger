use super::Pos;

/// テキスト上の範囲
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Range {
    pub(crate) start: Pos,
    pub(crate) end: Pos,
}

impl Range {
    pub(crate) fn start(&self) -> Pos {
        self.start
    }

    pub(crate) fn end(&self) -> Pos {
        self.end
    }

    pub(crate) fn is_touched(&self, pos: Pos) -> bool {
        self.start <= pos && pos <= self.end
    }

    pub(crate) fn unite(&self, other: &Self) -> Self {
        Range {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
