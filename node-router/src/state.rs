use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

#[derive(Clone)]
pub struct SharedState {
    term: Arc<AtomicBool>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            term: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn term_arc(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.term)
    }

    pub fn running(&self) -> bool {
        !self.term.load(Ordering::Relaxed)
    }
}
