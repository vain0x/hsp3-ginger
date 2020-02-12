use super::*;
use std::fmt;

/// ソースコード上の範囲。
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(crate) struct Range {
    pub(crate) start: Position,
    pub(crate) end: Position,
}

impl Range {
    pub(crate) fn new(start: Position, end: Position) -> Range {
        Range { start, end }
    }

    pub(crate) fn start(&self) -> Position {
        self.start
    }

    pub(crate) fn end(&self) -> Position {
        self.end
    }

    /// 指定された位置がこの範囲に含まれるか、あるいは範囲の終端を指しているとき true。
    pub(crate) fn contains_loosely(self, position: Position) -> bool {
        self.start <= position && position <= self.end
    }

    /// 2つの範囲を連結して1つの範囲にする。(2つの範囲を両方とも覆う最小の範囲を求める。)
    pub(crate) fn unite(self, other: Range) -> Range {
        Range {
            start: self.start.min(other.start),
            end: self.start.max(other.end),
        }
    }
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // GNU 形式 (https://www.gnu.org/prep/standards/html_node/Errors.html)
        write!(
            f,
            "{}.{}-{}.{}",
            self.start.line + 1,
            self.start.character + 1,
            self.end.line + 1,
            self.end.character + 1
        )
    }
}
