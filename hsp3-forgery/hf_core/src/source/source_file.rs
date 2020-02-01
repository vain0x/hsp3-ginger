use super::*;
use crate::framework::*;
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) type SourceFileId = Id<SourceFile>;

pub(crate) struct SourceFile {
    source_path: Rc<PathBuf>,
}

pub(crate) type SourceFileComponent = Component<SourceFile>;
