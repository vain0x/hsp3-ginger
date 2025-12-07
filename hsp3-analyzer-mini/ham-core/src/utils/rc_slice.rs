use super::rc_item::RcItem;
use std::{
    fmt::{self, Debug, Formatter},
    ops::{Deref, Index, Range, RangeFrom, RangeTo},
    rc::Rc,
};

/// 共有可能な配列を表す。
///
/// - データはヒープ上に確保される。(空の配列はメモリを確保しない。)
/// - 配列の要素への排他参照 (`&mut _`) は取れない。
/// - 配列の一部または全部を共有できる。
///     - 参照カウンタで管理されているため、共有データがなくなった時点で破棄される。
///     - クローンは参照を作るだけなので高速。
pub(crate) struct RcSlice<T> {
    repr: Repr<T>,
}

enum Repr<T> {
    Empty,
    NonEmpty { full: Rc<[T]>, start: u32, end: u32 },
}

impl<T> RcSlice<T> {
    /// 空のスライス
    pub(crate) const EMPTY: Self = RcSlice { repr: Repr::Empty };

    pub(crate) fn new(full: Rc<[T]>, start: usize, end: usize) -> Self {
        let n = full.len();
        assert!(start <= end && end <= n);

        if start >= end {
            Self::EMPTY
        } else {
            debug_assert!(start < n && start < end);
            RcSlice {
                repr: Repr::NonEmpty {
                    full,
                    start: start as u32,
                    end: end as u32,
                },
            }
        }
    }

    pub(crate) fn from_iter(iter: impl IntoIterator<Item = T>) -> Self {
        let items = Rc::from_iter(iter);
        let len = items.len();
        Self::new(items, 0, len)
    }

    /// 空か？
    pub(crate) fn is_empty(&self) -> bool {
        match self.repr {
            Repr::Empty => true,
            Repr::NonEmpty { .. } => false,
        }
    }

    /// 要素のスライスを借用する。`as_ref` と同じ。
    pub(crate) fn as_slice(&self) -> &[T] {
        match self.repr {
            Repr::Empty => &[],
            Repr::NonEmpty {
                ref full,
                start,
                end,
            } => &full[start as usize..end as usize],
        }
    }

    /// 一部の要素からなる配列を作る。(データは共有される。)
    pub(crate) fn slice(&self, start: usize, end: usize) -> Self {
        match self.repr {
            Repr::Empty => Self::EMPTY,
            Repr::NonEmpty {
                ref full,
                start: base_start,
                end: base_end,
            } => {
                let new_start = ((base_start as usize) + start).min(base_end as usize);
                let new_end = ((base_start as usize) + end).min(base_end as usize);
                RcSlice::new(full.clone(), new_start, new_end)
            }
        }
    }

    /// `index` 番目の要素への参照を作る。
    pub(crate) fn item(&self, index: usize) -> Option<RcItem<T>> {
        match self.repr {
            Repr::Empty => None,
            Repr::NonEmpty {
                ref full,
                start,
                end,
            } => {
                let i = (start as usize) + index;
                if i < (end as usize) {
                    Some(RcItem::new(full.clone(), i))
                } else {
                    None
                }
            }
        }
    }
}

impl<T: Clone> RcSlice<T> {
    /// 要素をすべてクローンしてベクタを作る。
    pub(crate) fn to_owned(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

impl<T> AsRef<[T]> for RcSlice<T> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

// `self.xxx` でスライスが持つメソッドを呼べるようにする。
impl<T> Deref for RcSlice<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

// `slice[i]`
impl<T> Index<usize> for RcSlice<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        &self.as_slice()[index]
    }
}

// `slice[start..end]`
impl<T> Index<Range<usize>> for RcSlice<T> {
    type Output = [T];

    fn index(&self, index: Range<usize>) -> &[T] {
        &self.as_slice()[index]
    }
}

// `slice[start..]`
impl<T> Index<RangeFrom<usize>> for RcSlice<T> {
    type Output = [T];

    fn index(&self, index: RangeFrom<usize>) -> &[T] {
        &self.as_slice()[index]
    }
}

// `slice[..end]`
impl<T> Index<RangeTo<usize>> for RcSlice<T> {
    type Output = [T];

    fn index(&self, index: RangeTo<usize>) -> &[T] {
        &self.as_slice()[index]
    }
}

impl<T: PartialEq> PartialEq<[T]> for RcSlice<T> {
    fn eq(&self, other: &[T]) -> bool {
        self.as_slice() == other
    }
}

impl<T: PartialEq> PartialEq for RcSlice<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for RcSlice<T> {}

// 必要なら PartialOrd, Ord も実装する。

impl<T: Debug> Debug for RcSlice<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        <[T] as Debug>::fmt(self.as_slice(), f)
    }
}

// `derive(Clone)` だと `T: Clone` のときしかCloneを実装しない。
impl<T> Clone for RcSlice<T> {
    fn clone(&self) -> Self {
        match self.repr {
            Repr::Empty => RcSlice::EMPTY,
            Repr::NonEmpty {
                ref full,
                start,
                end,
            } => RcSlice {
                repr: Repr::NonEmpty {
                    full: full.clone(),
                    start,
                    end,
                },
            },
        }
    }
}

impl<T> Default for RcSlice<T> {
    fn default() -> Self {
        RcSlice::EMPTY
    }
}

impl<T> From<[T; 0]> for RcSlice<T> {
    fn from(_: [T; 0]) -> Self {
        Self::EMPTY
    }
}

impl<T> From<Vec<T>> for RcSlice<T> {
    fn from(vec: Vec<T>) -> Self {
        Self::from_iter(vec.into_iter())
    }
}
