use std::fmt;

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Position {
    pub line: usize,
    pub character: usize,
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line + 1, self.character + 1)
    }
}

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

#[derive(Clone, Copy)]
pub(crate) struct SourceLocation {
    pub source_id: usize,
    pub range: Range,
}

impl SourceLocation {
    pub(crate) fn unite(self, other: SourceLocation) -> SourceLocation {
        SourceLocation {
            range: self.range.unite(other.range),
            ..self
        }
    }
}

impl fmt::Debug for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]({:?})", self.source_id, self.range)
    }
}
