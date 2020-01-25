use super::*;
use std::fmt;

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
