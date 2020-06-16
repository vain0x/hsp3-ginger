use std::borrow::Borrow;
use std::cmp::{min, Ordering};
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::{ops::Deref, rc::Rc};

#[derive(Clone)]
pub(crate) struct RcStr {
    full_text: Rc<String>,
    start: usize,
    end: usize,
}

impl RcStr {
    pub(crate) fn new(full_text: Rc<String>, start: usize, end: usize) -> Self {
        assert!(full_text.len() <= end);

        Self {
            full_text,
            start,
            end,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.end - self.start
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.full_text[self.start..self.end]
    }

    pub(crate) fn slice(&self, start: usize, end: usize) -> RcStr {
        Self {
            full_text: self.full_text.clone(),
            start: min(self.start + start, self.end),
            end: min(self.start + end, self.end),
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
        RcStr::from(std::rc::Rc::new(it))
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
