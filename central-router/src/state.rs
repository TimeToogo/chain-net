use std::{
    env,
    net::Ipv4Addr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::SystemTime,
};

use pnet::util::MacAddr;

#[derive(Clone)]
pub struct SharedState {
    term: Arc<AtomicBool>,
    state: Arc<Mutex<State>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Client {
    pub name: String,
    pub ip: Ipv4Addr,
    pub mac: Option<MacAddr>,
    pub created: SystemTime,
}

#[derive(Clone, Debug)]
pub struct State {
    pub on: bool,
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

    pub fn get<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&State) -> R,
    {
        let state = self.state.lock().unwrap();

        f(&*state)
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
            on: env::var("FORWARDER_ON").map(|_| true).unwrap_or(false),
            clients: Vec::new(),
        }
    }
}
