use super::*;
use crate::framework::*;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Source {
    pub(crate) source_id: Id<Source>,
    pub(crate) source_path: Rc<PathBuf>,
}

impl Source {
    pub(crate) fn new(source_id: Id<Source>, source_path: Rc<PathBuf>) -> Self {
        Source {
            source_id,
            source_path,
        }
    }
}

impl fmt::Debug for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // FIXME: env!("CARGO_MANIFEST_DIR") からの相対パスにしたい
        let short_path = self
            .source_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or("???".to_string());
        write!(f, "{}", short_path)
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.source_path.to_string_lossy())
    }
}

pub(crate) type SourceCodeComponent = HashMap<SyntaxSource, Rc<String>>;
