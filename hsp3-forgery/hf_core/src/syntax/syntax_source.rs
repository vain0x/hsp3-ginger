use super::*;
use std::fmt;

/// 構文木の出処となるもの。
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct SyntaxSource {
    pub(crate) source_file_id: SourceFileId,
    pub(crate) source_files: *const SourceFileComponent,
}

impl fmt::Debug for SyntaxSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match unsafe { &*self.source_files }.get(&self.source_file_id) {
            Some(source_file) => write!(f, "{:?}", source_file),
            None => write!(f, "???"),
        }
    }
}

impl fmt::Display for SyntaxSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match unsafe { &*self.source_files }.get(&self.source_file_id) {
            Some(source_file) => write!(f, "{}", source_file),
            None => write!(f, "???"),
        }
    }
}
