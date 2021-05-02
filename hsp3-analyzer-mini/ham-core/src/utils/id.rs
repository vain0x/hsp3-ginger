use std::cmp::{min, Ordering};
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

pub(crate) struct Id<T>(usize, PhantomData<T>);

impl<T> Id<T> {
    pub(crate) const fn new(id: usize) -> Self {
        Id(id, PhantomData)
    }

    pub(crate) fn get(self) -> usize {
        self.0
    }
}

impl<T> From<usize> for Id<T> {
    fn from(id: usize) -> Self {
        Id::new(id)
    }
}

impl<T> From<Id<T>> for usize {
    fn from(id: Id<T>) -> usize {
        id.0
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Id::new(self.0)
    }
}

impl<T> Copy for Id<T> {}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Id::new(usize::default())
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Id<T> {}

impl<T> Debug for Id<T> {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        Debug::fmt(&self.0, formatter)
    }
}

impl<T> Display for Id<T> {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> std::ops::Add<usize> for Id<T> {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Id::from(self.0 + rhs)
    }
}

impl<T> std::ops::AddAssign<usize> for Id<T> {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl<T> std::ops::Sub<usize> for Id<T> {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self {
        Id::from(self.0 - min(self.0, rhs))
    }
}
