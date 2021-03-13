mod nodes;
mod status;
mod ui;

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

    let api_nodes = warp::path!("api" / "nodes").and(
        nodes::get(state.clone())
            .or(nodes::post(state.clone()))
            .or(nodes::put(state.clone()))
            .or(nodes::delete(state.clone())),
    );

    warp::serve(api_status.or(api_nodes).or(ui::get()))
        .run(([0, 0, 0, 0], args.port))
        .await;

    Ok(())
}

async fn stopped_running(state: SharedState) {
    while state.running() {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
