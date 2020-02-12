use super::*;
use std::fmt;

/// トークン列の出処となるもの。
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct TokenSource {
    pub(crate) source_file: SourceFile,
}

impl TokenSource {
    pub(crate) fn from_file(source_file: SourceFile) -> Self {
        TokenSource { source_file }
    }
}

impl fmt::Debug for TokenSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.source_file)
    }
}

impl fmt::Display for TokenSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.source_file)
    }
}
