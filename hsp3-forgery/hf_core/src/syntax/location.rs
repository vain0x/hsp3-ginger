use super::*;
use crate::framework::*;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) struct Location {
    pub(crate) source_id: Id<Source>,
    pub(crate) source_path: Rc<PathBuf>,
    pub(crate) range: Range,
}

impl Location {
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
        // FIXME: env!("CARGO_MANIFEST_DIR") からの相対パスにしたい
        let short_path = self
            .source_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or("???".to_string());

        write!(f, "{}:{}", short_path, self.range)
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.source_path.to_string_lossy(), self.range)
    }
}
