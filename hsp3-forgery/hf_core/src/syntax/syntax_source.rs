use super::*;
use std::fmt;

/// 構文木の出処となるもの。
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct SyntaxSource {
    pub(crate) source: Source,
}

impl fmt::Debug for SyntaxSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.source)
    }
}

impl fmt::Display for SyntaxSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}
