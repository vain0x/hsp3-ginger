use super::*;
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) type SourceFileId = Id<SourceFile>;

#[derive(Clone)]
pub(crate) struct SourceFile {
    source_path: Rc<PathBuf>,
}
