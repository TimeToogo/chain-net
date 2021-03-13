use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};
use warp::{filters::BoxedFilter, hyper::StatusCode, Filter, Reply};

use crate::state::{Node, SharedState};

#[derive(Serialize)]
struct NodeResponse {
    name: String,
    ip: String,
    mac: Option<String>,
    created: SystemTime,
    you: bool,
}

#[derive(Deserialize)]
struct NewNode {
    name: String,
}

#[derive(Deserialize)]
struct ReorderRequest {
    cur_i: usize,
    new_i: usize
}

impl From<&Node> for NodeResponse {
    fn from(c: &Node) -> Self {
        Self {
            name: c.name.clone(),
            ip: c.ip.to_string(),
            mac: c.mac.map(|i| i.to_string()),
            created: c.created,
            you: false,
        }
    }
}

pub fn get(state: SharedState) -> BoxedFilter<(impl Reply,)> {
    warp::get()
        .and(warp::addr::remote())
        .map(move |addr: Option<SocketAddr>| warp::reply::json(&get_nodes(&state, addr)))
        .boxed()
}

pub fn post(state: SharedState) -> BoxedFilter<(impl Reply,)> {
    warp::post()
        .and(warp::addr::remote())
        .and(warp::body::json())
        .map(move |addr: Option<SocketAddr>, n: NewNode| {
            let ip = match get_client_ip(addr) {
                Some(addr) => addr,
                None => return StatusCode::BAD_REQUEST,
            };

            upsert_node(&state, n, ip);
            StatusCode::OK
        })
        .boxed()
}

pub fn put(state: SharedState) -> BoxedFilter<(impl Reply,)> {
    warp::put()
        .and(warp::body::json())
        .map(move |req: ReorderRequest| {
            reorder_node(&state, req);
            StatusCode::OK
        })
        .boxed()
}

pub fn delete(state: SharedState) -> BoxedFilter<(impl Reply,)> {
    warp::delete()
        .and(warp::addr::remote())
        .map(move |addr: Option<SocketAddr>| {
            let ip = match get_client_ip(addr) {
                Some(addr) => addr,
                None => return StatusCode::BAD_REQUEST,
            };

            delete_node(&state, ip);
            StatusCode::OK
        })
        .boxed()
}

fn get_client_ip(addr: Option<SocketAddr>) -> Option<Ipv4Addr> {
    if addr.is_none() {
        log::error!("connection does not have socket");
        return None;
    }

    let ip = addr.unwrap().ip();

    let ip = match ip {
        IpAddr::V4(ip) => Some(ip),
        _ => {
            log::error!("ipv6 address not supported, node has addr {}", ip);
            None
        }
    };

    if let Some(ip) = ip {
        if ip.is_loopback() || ip.is_broadcast() {
            log::error!("node ip cannot be loopback or broadcast");
            return None;
        }
    }

    return ip;
}

fn get_nodes(state: &SharedState, client_addr: Option<SocketAddr>) -> Vec<NodeResponse> {
    let client_ip = get_client_ip(client_addr)
        .map(|i| i.to_string())
        .unwrap_or("".to_string());

    state
        .get(|s| s.nodes.clone())
        .iter()
        .map(NodeResponse::from)
        .map(move |mut i| {
            i.you = i.ip == client_ip;
            i
        })
        .collect::<Vec<_>>()
}

fn upsert_node(state: &SharedState, n: NewNode, ip: Ipv4Addr) {
    state.update(|state| {
        if let Some(node) = state.nodes.iter_mut().filter(|i| i.ip == ip).next() {
            node.name = n.name
        } else {
            let node = Node {
                name: n.name.clone(),
                ip,
                mac: None,
                created: SystemTime::now(),
            };
            log::info!("added node {:?}", node);
            state.nodes.push(node);
        }
    })
}

fn delete_node(state: &SharedState, ip: Ipv4Addr) -> () {
    state.update(|s| s.nodes.retain(|i| i.ip != ip))
}

fn reorder_node(state: &SharedState, req: ReorderRequest) {
    state.update(|s| {
        if req.cur_i >= s.nodes.len() {
            log::error!("reorder: invalid cur idx {}", req.cur_i);
            return;
        }

        if req.new_i >= s.nodes.len() {
            log::error!("reorder: invalid new idx {}", req.new_i);
            return;
        }
        
        let node = s.nodes.remove(req.cur_i);
        s.nodes.insert(req.new_i, node);
    });
}