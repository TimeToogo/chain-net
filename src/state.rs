use std::{
    net::Ipv4Addr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use pnet::util::MacAddr;

#[derive(Clone)]
pub struct SharedState {
    term: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
}

#[derive(Clone, Debug)]
pub struct Client {
    pub name: String,
    pub ip: Ipv4Addr,
    pub mac: Option<MacAddr>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub clients: Vec<Client>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            term: Arc::new(AtomicBool::new(false)),
            state: Arc::new(Mutex::new(State::new())),
        }
    }

    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut State) -> (),
    {
        let mut state = self.state.lock().unwrap();

        f(&mut *state);
    }

    pub fn inner_clone(&self) -> State {
        let state = self.state.lock().unwrap();

        (*state).clone()
    }

    pub fn term_arc(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.term)
    }

    pub fn running(&self) -> bool {
        !self.term.load(Ordering::Relaxed)
    }
}

impl State {
    fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }
}
