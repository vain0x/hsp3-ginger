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
    NonEmpty {
        full_text: Rc<String>,
        start: usize,
        end: usize,
    },
}

impl RcStr {
    pub(crate) const EMPTY: RcStr = RcStr { repr: Repr::Empty };

    pub(crate) fn new(full_text: Rc<String>, start: usize, end: usize) -> Self {
        assert!(full_text.is_char_boundary(start));
        assert!(full_text.is_char_boundary(end));

        // 文字境界の判定が配列の境界判定も兼ねているので、実行時に検査しなくてもいい。
        debug_assert!(start <= end);
        debug_assert!(end <= full_text.len());

        if start >= end {
            RcStr::EMPTY
        } else {
            debug_assert!(start < full_text.len() && start < end);

            RcStr {
                repr: Repr::NonEmpty {
                    full_text,
                    start,
                    end,
                },
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self.repr {
            Repr::Empty => 0,
            Repr::NonEmpty { start, end, .. } => {
                debug_assert!(start < end);
                end - start
            }
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        match self.repr {
            Repr::Empty => "",
            Repr::NonEmpty {
                ref full_text,
                start,
                end,
            } => &full_text[start..end],
        }
    }

    pub(crate) fn slice(&self, start: usize, end: usize) -> RcStr {
        match self.repr {
            Repr::Empty => Self::EMPTY,
            Repr::NonEmpty {
                full_text: ref underlying,
                start: base_start,
                end: base_end,
            } => {
                let new_start = (base_start + start).min(base_end);
                let new_end = (base_start + end).min(base_end);
                RcStr::new(underlying.clone(), new_start, new_end)
            }
        }
    }

    pub(crate) fn to_string(&self) -> String {
        self.as_str().to_string()
    }
}

impl From<Rc<String>> for RcStr {
    fn from(it: Rc<String>) -> RcStr {
        let len = it.len();
        RcStr::new(it, 0, len)
    }
}

impl From<String> for RcStr {
    fn from(it: String) -> RcStr {
        RcStr::from(Rc::new(it))
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
