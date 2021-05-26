use super::{range_is_touched, DocId, Pos, Pos16, Range};

/// Location. テキストドキュメント上の範囲
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Loc {
    pub(crate) doc: DocId,
    pub(crate) range: Range,
}

impl Loc {
    pub(crate) fn from_doc(doc: DocId) -> Self {
        Loc {
            doc,
            range: Range::empty(Pos::default()),
        }
    }

    pub(crate) fn new3(doc: DocId, start: Pos, end: Pos) -> Self {
        Loc {
            doc,
            range: Range::from(start..end),
        }
    }

    pub(crate) fn with_range(self, range: Range) -> Self {
        Loc { range, ..self }
    }

    pub(crate) fn start(&self) -> Pos {
        self.range.start()
    }

    pub(crate) fn end(&self) -> Pos {
        self.range.end()
    }

    #[cfg(unused)]
    pub(crate) fn start_row(&self) -> usize {
        self.range.start().row as usize
    }

    #[cfg(unused)]
    pub(crate) fn end_row(&self) -> usize {
        self.range.end().row as usize
    }

    pub(crate) fn ahead(&self) -> Loc {
        Loc {
            doc: self.doc,
            range: Range::empty(self.start()),
        }
    }

    pub(crate) fn behind(&self) -> Loc {
        Loc {
            doc: self.doc,
            range: Range::empty(self.end()),
        }
    }

    pub(crate) fn is_touched(&self, doc: DocId, pos: Pos16) -> bool {
        self.doc == doc && range_is_touched(&self.range, pos)
    }

    pub(crate) fn unite(&self, other: &Self) -> Loc {
        if self.doc != other.doc {
            return *self;
        }

        Loc {
            doc: self.doc,
            range: self.range.join(other.range),
        }
    }
}
