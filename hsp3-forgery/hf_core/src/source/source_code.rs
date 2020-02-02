use super::*;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) type SourceCode = Rc<String>;

pub(crate) type SourceCodeComponent = HashMap<SourceFileId, SourceCode>;
