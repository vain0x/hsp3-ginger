use super::*;
use std::fmt;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub(crate) fn unite(self, other: Range) -> Range {
        Range {
            start: self.start.min(other.start),
            end: self.start.max(other.end),
        }
    }
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}..{:?}", self.start, self.end)
    }
}
