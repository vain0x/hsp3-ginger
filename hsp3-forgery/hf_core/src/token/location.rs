use super::*;
use std::fmt;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Location {
    pub(crate) source: TokenSource,
    pub(crate) range: Range,
}

impl Location {
    pub(crate) fn new(source: TokenSource, range: Range) -> Location {
        Location { source, range }
    }

    pub(crate) fn range(&self) -> Range {
        self.range
    }

    pub(crate) fn start(&self) -> Position {
        self.range.start
    }

    pub(crate) fn unite(self, other: &Location) -> Location {
        Location {
            range: self.range.unite(other.range),
            ..self
        }
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{}", self.source, self.range)
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.source, self.range)
    }
}
