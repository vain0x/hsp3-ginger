use crate::id::Id;
use std::rc::Rc;

pub(crate) type DocId = Id<Doc>;

#[derive(Clone, Debug)]
pub(crate) struct Doc {
    pub(crate) id: DocId,
    pub(crate) text: Rc<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Pos {
    pub(crate) row: usize,
    pub(crate) col: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Loc {
    pub(crate) doc: DocId,
    pub(crate) start: Pos,
    pub(crate) end: Pos,
}
