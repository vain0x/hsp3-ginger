use super::{ADoc, APos, ARange};

/// Location. テキストドキュメント上の範囲
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ALoc {
    pub(crate) doc: ADoc,
    pub(crate) range: ARange,
}

impl ALoc {
    pub(crate) fn new3(doc: ADoc, start: APos, end: APos) -> Self {
        ALoc {
            doc,
            range: ARange { start, end },
        }
    }

    pub(crate) fn start(&self) -> APos {
        self.range.start()
    }

    #[allow(unused)]
    pub(crate) fn end(&self) -> APos {
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

    pub(crate) fn is_touched(&self, doc: ADoc, pos: APos) -> bool {
        self.doc == doc && self.range.is_touched(pos)
    }

    pub(crate) fn behind(&self) -> ALoc {
        ALoc {
            doc: self.doc,
            range: ARange {
                start: self.end(),
                end: self.end(),
            },
        }
    }
}
