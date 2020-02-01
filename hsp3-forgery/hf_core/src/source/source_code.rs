use super::*;
use crate::framework::*;
use std::path::PathBuf;
use std::rc::Rc;

pub(crate) type SourceCodeId = Id<SourceCode>;

pub(crate) type SourceCode = Rc<String>;

pub(crate) type SourceCodeComponent = Component<SourceCode>;

pub(crate) type SourceCodeProperty = HashMap<SourceCodeHolder, SourceCodeId>;
