use super::*;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct SourceLocation {
    pub(crate) source_id: usize,
    pub(crate) source_path: Rc<PathBuf>,
    pub(crate) range: Range,
}

impl SourceLocation {
    pub(crate) fn start(&self) -> Position {
        self.range.start
    }

    pub(crate) fn unite(self, other: &SourceLocation) -> SourceLocation {
        SourceLocation {
            range: self.range.unite(other.range),
            ..self
        }
    }
}

impl fmt::Debug for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}:{:?}", self.source_path, self.range)
    }
}
