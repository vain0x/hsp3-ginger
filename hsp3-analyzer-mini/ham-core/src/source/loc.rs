use super::{DocId, Pos, Range};

/// Location. テキストドキュメント上の範囲
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Loc {
    pub(crate) doc: DocId,
    pub(crate) range: Range,
}

impl Loc {
    pub(crate) fn new3(doc: DocId, start: Pos, end: Pos) -> Self {
        Loc {
            doc,
            range: Range { start, end },
        }
    }

    pub(crate) fn start(&self) -> Pos {
        self.range.start()
    }

    #[allow(unused)]
    pub(crate) fn end(&self) -> Pos {
        self.range.end()
    }

    pub(crate) fn start_row(&self) -> usize {
        self.range.start.row
    }

    pub(crate) fn start_column(&self) -> usize {
        self.range.start.column
    }

    pub(crate) fn end_row(&self) -> usize {
        self.range.end.row
    }

    pub(crate) fn end_column(&self) -> usize {
        self.range.end.column
    }

    pub(crate) fn is_touched(&self, doc: DocId, pos: Pos) -> bool {
        self.doc == doc && self.range.is_touched(pos)
    }

    pub(crate) fn ahead(&self) -> Loc {
        Loc {
            doc: self.doc,
            range: Range {
                start: self.start(),
                end: self.end(),
            },
        }
    }

    pub(crate) fn behind(&self) -> Loc {
        Loc {
            doc: self.doc,
            range: Range {
                start: self.end(),
                end: self.end(),
            },
        }
    }

    pub(crate) fn unite(&self, other: &Self) -> Loc {
        if self.doc != other.doc {
            return *self;
        }

        Loc {
            doc: self.doc,
            range: self.range.unite(&other.range),
        }
    }
}
