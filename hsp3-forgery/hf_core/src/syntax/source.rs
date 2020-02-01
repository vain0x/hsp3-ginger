use super::*;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) type SourceCodeComponent = HashMap<SyntaxSource, Rc<String>>;
