use super::*;

#[derive(Default)]
pub(crate) struct IdProvider {
    last_id: usize,
}

impl IdProvider {
    pub(crate) fn new() -> Self {
        IdProvider::default()
    }

    pub(crate) fn fresh<T>(&mut self) -> Id<T> {
        self.last_id += 1;
        Id::new(self.last_id)
    }
}
