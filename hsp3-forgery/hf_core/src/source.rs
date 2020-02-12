pub(crate) mod position;
pub(crate) mod range;
pub(crate) mod source_file;

pub(crate) use crate::framework::*;
pub(crate) use position::Position;
pub(crate) use range::Range;
pub(crate) use source_file::*;

pub(crate) type SourceCode = String;
