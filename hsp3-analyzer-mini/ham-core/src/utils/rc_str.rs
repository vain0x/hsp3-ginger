use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Deref,
    rc::Rc,
};

/// 共有可能な文字列。(`RcSlice` の文字列版。)
#[derive(Clone)]
pub(crate) struct RcStr {
    repr: Repr,
}

#[derive(Clone)]
enum Repr {
    Empty,
    NonEmpty { full: Rc<str>, start: u32, end: u32 },
}

impl RcStr {
    pub(crate) const EMPTY: RcStr = RcStr { repr: Repr::Empty };

    pub(crate) fn new(full: Rc<str>, start: usize, end: usize) -> Self {
        assert!(full.is_char_boundary(start));
        assert!(full.is_char_boundary(end));

        // 文字境界の判定が配列の境界判定も兼ねているので、実行時に検査しなくてもいい。
        debug_assert!(start <= end);
        debug_assert!(end <= full.len());

        if start >= end {
            RcStr::EMPTY
        } else {
            debug_assert!(start < full.len() && start < end);

            RcStr {
                repr: Repr::NonEmpty {
                    full,
                    start: start as u32,
                    end: end as u32,
                },
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self.repr {
            Repr::Empty => 0,
            Repr::NonEmpty { start, end, .. } => (end - start) as usize,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self.repr {
            Repr::Empty => "",
            Repr::NonEmpty {
                ref full,
                start,
                end,
            } => &full[start as usize..end as usize],
        }
    }

    pub(crate) fn slice(&self, start: usize, end: usize) -> RcStr {
        match self.repr {
            Repr::Empty => Self::EMPTY,
            Repr::NonEmpty {
                ref full,
                start: base_start,
                end: base_end,
            } => {
                let new_start = ((base_start as usize) + start).min(base_end as usize);
                let new_end = ((base_start as usize) + end).min(base_end as usize);
                RcStr::new(full.clone(), new_start, new_end)
            }
        }
    }

    pub(crate) fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

impl From<Rc<str>> for RcStr {
    fn from(it: Rc<str>) -> RcStr {
        let len = it.len();
        RcStr::new(it, 0, len)
    }
}

impl From<String> for RcStr {
    fn from(it: String) -> RcStr {
        RcStr::from(Rc::from(it.as_ref()))
    }
}

impl<'a> From<&'a str> for RcStr {
    fn from(it: &'a str) -> RcStr {
        if it.is_empty() {
            RcStr::EMPTY
        } else {
            RcStr::from(Rc::from(it))
        }
    }
}

impl AsRef<str> for RcStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for RcStr {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Hash for RcStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl PartialEq<str> for RcStr {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq for RcStr {
    fn eq(&self, other: &RcStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for RcStr {}

impl PartialOrd<str> for RcStr {
    fn partial_cmp(&self, other: &str) -> Option<Ordering> {
        self.as_str().partial_cmp(other)
    }
}

impl PartialOrd for RcStr {
    fn partial_cmp(&self, other: &RcStr) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl Ord for RcStr {
    fn cmp(&self, other: &RcStr) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl Debug for RcStr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        <str as Debug>::fmt(self.as_str(), f)
    }
}

impl Display for RcStr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        <str as Display>::fmt(self.as_str(), f)
    }
}

impl Deref for RcStr {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl Default for RcStr {
    fn default() -> Self {
        RcStr::EMPTY
    }
}
