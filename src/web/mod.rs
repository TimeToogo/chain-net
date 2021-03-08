use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, time::Duration};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use warp::{hyper::StatusCode, Filter};

use crate::{
    args::Args,
    state::{Client, SharedState},
};

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

#[derive(Serialize)]
struct ClientResponse {
    name: String,
    ip: String,
    mac: Option<String>,
}

#[derive(Deserialize)]
struct NewClient {
    name: String,
}

impl From<&Client> for ClientResponse {
    fn from(c: &Client) -> Self {
        Self {
            name: c.name.clone(),
            ip: c.ip.to_string(),
            mac: c.mac.map(|i| i.to_string()),
        }
    }
}

async fn start_warp(args: Args, state: SharedState) -> Result<()> {
    let get_clients = {
        let state = state.clone();
        warp::get().map(move || {
            warp::reply::json(&get_clients(&state))
        })
    };

    let post_clients = {
        let state = state.clone();
        warp::post()
            .and(warp::addr::remote())
            .and(warp::body::json())
            .map(move |addr: Option<SocketAddr>, n: NewClient| {
                if addr.is_none() {
                    log::error!("connection does not have socket");
                    return StatusCode::BAD_REQUEST;
                }

                let ip = addr.unwrap().ip();

                let ip = match ip {
                    IpAddr::V4(ip) => ip,
                    _ => {
                        log::error!("ipv6 address not supported, client has addr {}", ip);
                        return StatusCode::BAD_REQUEST;
                    }
                };

                upsert_client(&state, n, ip);
                StatusCode::OK
            })
    };

    let api_clients = warp::path!("api" / "clients").and(get_clients.or(post_clients));

    warp::serve(api_clients)
        .run(([0, 0, 0, 0], args.port))
        .await;

    Ok(())
}

fn get_clients(state: &SharedState) -> Vec<ClientResponse> {
    state
        .inner_clone()
        .clients
        .iter()
        .map(ClientResponse::from)
        .collect::<Vec<_>>()
}

fn upsert_client(state: &SharedState, n: NewClient, ip: Ipv4Addr) {
    state.update(|state| {
        if let Some(client) = state.clients.iter_mut().filter(|i| i.ip == ip).next() {
            client.name = n.name
        } else {
            let client = Client {
                name: n.name.clone(),
                ip,
                mac: None,
            };
            log::info!("added client {:?}", client);
            state.clients.push(client);
        }
    })
}

async fn stopped_running(state: SharedState) {
    while state.running() {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
