use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
    rc::Rc,
};

/// 配列の要素への共有可能な参照。
///
/// - `Rc<T>` または `RcSlice<T>` によって所有されている配列に含まれる、1つの要素を指す。
/// - データは共有可能。クローンは高速。排他参照は取れない。
pub(crate) struct RcItem<T> {
    underlying: Rc<[T]>,
    index: usize,
}

impl<T> RcItem<T> {
    pub(crate) fn new(underlying: Rc<[T]>, index: usize) -> Self {
        assert!(index < underlying.len());
        RcItem { underlying, index }
    }

    pub(crate) fn new_single(value: T) -> Self {
        RcItem::new(Rc::new([value]), 0)
    }
}

#[cfg(unused)]
impl<T: Clone> RcItem<T> {
    pub(crate) fn to_owned(&self) -> T {
        self.as_ref().clone()
    }
}

impl<T> AsRef<T> for RcItem<T> {
    fn as_ref(&self) -> &T {
        &self.underlying[self.index]
    }
}

// `self.xxx` で `T` が持つメソッドを呼べるようにする。
impl<T> Deref for RcItem<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}

impl<T: PartialEq> PartialEq<T> for RcItem<T> {
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == other
    }
}

impl<T: PartialEq> PartialEq for RcItem<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl<T: Eq> Eq for RcItem<T> {}

// 必要なら PartialOrd, Ord も実装する。

impl<T: Debug> Debug for RcItem<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        <T as Debug>::fmt(self.as_ref(), f)
    }
}

// `derive(Clone)` だと `T: Clone` のときしかCloneを実装しない。
impl<T> Clone for RcItem<T> {
    fn clone(&self) -> Self {
        RcItem {
            underlying: self.underlying.clone(),
            index: self.index,
        }
    }
}
