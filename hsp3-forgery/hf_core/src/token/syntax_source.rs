use super::*;
use std::fmt;

/// 構文木の出処となるもの。
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct SyntaxSource {
    pub(crate) source_file: SourceFile,
}

impl SyntaxSource {
    pub(crate) fn from_file(source_file: SourceFile) -> Self {
        SyntaxSource { source_file }
    }
}

impl fmt::Debug for SyntaxSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.source_file)
    }
}

impl fmt::Display for SyntaxSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.source_file)
    }
}
