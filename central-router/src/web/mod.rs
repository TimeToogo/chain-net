mod clients;
mod status;

use std::time::Duration;

use anyhow::Result;
use warp::Filter;

use crate::{args::Args, state::SharedState};

pub fn start(args: Args, state: SharedState) -> Result<()> {
    log::info!("starting web server on port {}", args.port);

    tokio::runtime::Runtime::new()?.block_on(async move {
        tokio::select! {
            _ = start_warp(args, state.clone()) => (),
            _ = stopped_running(state) => ()
        }
    });

    log::info!("shutting down web server");

    Ok(())
}

async fn start_warp(args: Args, state: SharedState) -> Result<()> {
    let api_status = warp::path!("api" / "status")
        .and(status::get(state.clone()).or(status::post(state.clone())));

    let api_clients = warp::path!("api" / "clients")
        .and(clients::get(state.clone()).or(clients::post(state.clone())));

    warp::serve(api_status.or(api_clients))
        .run(([0, 0, 0, 0], args.port))
        .await;

    Ok(())
}

async fn stopped_running(state: SharedState) {
    while state.running() {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
