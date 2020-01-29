#[derive(Default)]
pub(crate) struct IdProvider {
    last_id: usize,
}

impl IdProvider {
    pub(crate) fn new() -> Self {
        IdProvider::default()
    }

    pub(crate) fn fresh(&mut self) -> usize {
        self.last_id += 1;
        self.last_id
    }
}
