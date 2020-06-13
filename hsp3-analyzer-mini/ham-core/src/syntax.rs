use crate::utils::id::Id;

pub(crate) type DocId = Id<Doc>;

pub(crate) enum Doc {}

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
