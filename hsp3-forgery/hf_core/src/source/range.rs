use super::*;
use std::fmt;

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) struct Range {
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl Range {
    pub(crate) fn new(start: Position, end: Position) -> Range {
        Range { start, end }
    }

    pub(crate) fn contains_loosely(self, position: Position) -> bool {
        self.start <= position && position <= self.end
    }

    pub(crate) fn unite(self, other: Range) -> Range {
        Range {
            start: self.start.min(other.start),
            end: self.start.max(other.end),
        }
    }
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // GNU 形式 (https://www.gnu.org/prep/standards/html_node/Errors.html)
        write!(
            f,
            "{}.{}-{}.{}",
            self.start.line + 1,
            self.start.character + 1,
            self.end.line + 1,
            self.end.character + 1
        )
    }
}
