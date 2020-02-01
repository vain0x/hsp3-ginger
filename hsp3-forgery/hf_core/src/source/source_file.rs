use super::*;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) type SourceFileId = Id<SourceFile>;

pub(crate) struct SourceFile {
    pub(crate) source_path: Rc<PathBuf>,
}

impl fmt::Debug for SourceFile {
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

impl fmt::Display for SourceFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.source_path.to_string_lossy())
    }
}

pub(crate) type SourceFileComponent = HashMap<SourceFileId, SourceFile>;
