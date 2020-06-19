use super::APos;

/// テキスト上の範囲
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ARange {
    pub(crate) start: APos,
    pub(crate) end: APos,
}

impl ARange {
    pub(crate) fn start(&self) -> APos {
        self.start
    }

    pub(crate) fn end(&self) -> APos {
        self.end
    }

    pub(crate) fn is_touched(&self, pos: APos) -> bool {
        self.start <= pos && pos <= self.end
    }

    pub(crate) fn unite(&self, other: &Self) -> Self {
        ARange {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
