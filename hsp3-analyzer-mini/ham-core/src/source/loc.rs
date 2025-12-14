use super::{range_is_touched, DocId, Pos, Pos16, Range};
use std::fmt::{self, Debug};

/// Location. テキストドキュメント上の範囲
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Loc {
    pub(crate) doc: DocId,
    pub(crate) range: Range,
}

impl Loc {
    /// ドキュメントの先頭を指す Loc を作る
    pub(crate) fn from_doc(doc: DocId) -> Self {
        Loc {
            doc,
            range: Range::empty(Pos::default()),
        }
    }

    pub(crate) fn new(doc: DocId, range: Range) -> Self {
        Loc { doc, range }
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

    /// 開始位置
    pub(crate) fn start(&self) -> Pos {
        self.range.start()
    }

    /// 終了位置
    pub(crate) fn end(&self) -> Pos {
        self.range.end()
    }

    #[allow(unused)]
    pub(crate) fn start_row(&self) -> usize {
        self.range.start().row as usize
    }

    #[allow(unused)]
    pub(crate) fn end_row(&self) -> usize {
        self.range.end().row as usize
    }

    /// 始端 (範囲の先頭を指す空の範囲に置き換え)
    pub(crate) fn ahead(&self) -> Loc {
        Loc {
            doc: self.doc,
            range: Range::empty(self.start()),
        }
    }

    /// 終端 (範囲の末尾を指す空の範囲に置き換え)
    pub(crate) fn behind(&self) -> Loc {
        Loc {
            doc: self.doc,
            range: Range::empty(self.end()),
        }
    }

    /// 指定位置を含むか
    pub(crate) fn is_touched(&self, doc: DocId, pos: Pos16) -> bool {
        self.doc == doc && range_is_touched(&self.range, pos)
    }

    /// 範囲を併合する
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

impl Debug for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Loc({}:{:?})", self.doc, self.range)
    }
}
