use crate::id::Id;
use std::rc::Rc;

pub(crate) type DocId = Id<Doc>;

#[derive(Clone, Debug)]
pub(crate) struct Doc {
    pub id: DocId,
    pub text: Rc<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Pos {
    pub row: usize,
    pub col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Loc {
    pub doc: DocId,
    pub start: Pos,
    pub end: Pos,
}
