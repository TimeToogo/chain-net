mod args;
mod eth;
pub mod state;
mod web;

use std::{process, thread};

use args::Args;
use clap::Clap;
use state::SharedState;
use thread::JoinHandle;
use anyhow::{Result, anyhow};

fn main() {
    env_logger::init_from_env(env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"));

    let args = Args::parse();

    let state = SharedState::new();

    log::info!("starting up");

    for sig in &[libc::SIGINT, libc::SIGTERM, libc::SIGQUIT] {
        signal_hook::flag::register(*sig, state.term_arc()).expect("failed to set signal handler");
    }

    let threads = vec![
        spawn(&args, &state, eth::start),
        spawn(&args, &state, web::start),
    ];

    let error = threads
        .into_iter()
        .map(|i| i.join().unwrap_or(Err(anyhow!("failed to join thread"))))
        .filter(|i| i.is_err())
        .map(|i| i.unwrap_err())
        .next();

    if error.is_some() {
        log::error!("error: {}", error.as_ref().unwrap());
    }

    log::info!("shutdown");

    process::exit(error.is_some() as _);
}

fn spawn<F>(args: &Args, state: &SharedState, f: F) -> JoinHandle<Result<()>> 
    where F : FnOnce(Args, SharedState) -> Result<()> + Send + 'static
{
    let args = args.clone();
    let state = state.clone();
    thread::spawn(move || f(args, state))
}
