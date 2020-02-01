use super::*;
use std::collections::HashMap;

pub(crate) struct Component<T> {
    last_id: usize,
    inner: HashMap<Id<T>, T>,
}

impl<T> Component<T> {
    pub(crate) fn fresh(&mut self) -> Id<T> {
        self.last_id += 1;
        Id::new(self.last_id)
    }

    pub(crate) fn get(&self, id: Id<T>) -> Option<&T> {
        self.inner.get(&id)
    }

    pub(crate) fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        self.inner.get_mut(&id)
    }

    pub(crate) fn set(&mut self, id: Id<T>, value: T) -> Option<T> {
        self.inner.insert(id, value)
    }

    pub(crate) fn unset(&mut self, id: Id<T>) -> Option<T> {
        self.inner.remove(&id)
    }

    pub(crate) fn iter<'a>(&'a self) -> impl Iterator<Item = (Id<T>, &'a T)> + 'a {
        self.inner.iter().map(|(id, value)| (*id, value))
    }

    pub(crate) fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (Id<T>, &'a mut T)> + 'a {
        self.inner.iter_mut().map(|(id, value)| (*id, value))
    }
}
