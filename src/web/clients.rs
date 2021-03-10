use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};
use warp::{filters::BoxedFilter, hyper::StatusCode, Filter, Reply};

use crate::state::{Client, SharedState};

#[derive(Serialize)]
struct ClientResponse {
    name: String,
    ip: String,
    mac: Option<String>,
    created: SystemTime,
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
            created: c.created,
        }
    }
}

pub fn get(state: SharedState) -> BoxedFilter<(impl Reply,)> {
    warp::get()
        .map(move || warp::reply::json(&get_clients(&state)))
        .boxed()
}

pub fn post(state: SharedState) -> BoxedFilter<(impl Reply,)> {
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

            if ip.is_loopback() || ip.is_broadcast() {
                log::error!("client ip cannot be loopback or broadcast");
                return StatusCode::BAD_REQUEST;
            }

            upsert_client(&state, n, ip);
            StatusCode::OK
        })
        .boxed()
}

fn get_clients(state: &SharedState) -> Vec<ClientResponse> {
    state
        .get(|s| s.clients.clone())
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
                created: SystemTime::now(),
            };
            log::info!("added client {:?}", client);
            state.clients.push(client);
        }
    })
}
