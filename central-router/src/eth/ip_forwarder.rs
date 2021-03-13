use std::{
    net::{IpAddr, Ipv4Addr},
    sync::mpsc::Sender,
};

use pnet::packet::ipv4::Ipv4Packet;
use pnet::{
    datalink::NetworkInterface,
    packet::ethernet::{EthernetPacket, MutableEthernetPacket},
    packet::Packet,
};
use state::{Node, State};

use crate::state::{self, SharedState};

use super::event::Event;

pub fn process_packet(
    tx: &mut Sender<Event>,
    state: &mut SharedState,
    eth: EthernetPacket,
    interface: &NetworkInterface,
) {
    let ip = Ipv4Packet::new(eth.payload()).unwrap();
    log::trace!("packet is ipv4");

    if !state.get(|s| s.on) {
        log::trace!("ignoring, state is off");
        return;
    }

    let source_mac = eth.get_source();
    let dest_ip = ip.get_destination();

    log::trace!("packet from {} to dest {}", source_mac, dest_ip);

    if !is_in_local_net(dest_ip, interface) {
        log::trace!("packet dest is not local network, ignoring");
        return;
    }

    if interface.ips.iter().any(|i| i.ip() == IpAddr::V4(dest_ip)) {
        log::trace!("packet dest is local interface, ignoring");
        return;
    }

    if source_mac == interface.mac.unwrap() {
        log::trace!("packet is sent from interface, ignoring");
        return;
    }

    log::trace!(
        "received packet from {} destined for {} in lan",
        source_mac,
        dest_ip
    );

    let nodes = state.get(|state| {
        let source_node = state.nodes.iter().find(|i| i.mac == Some(source_mac));

        if source_node.is_none() {
            log::trace!("could not find node with mac {}", source_mac);
            return None;
        }

        let dest_node = state.nodes.iter().find(|i| i.ip == dest_ip);

        if dest_node.is_none() {
            log::debug!("could not find dest node with ip {}", dest_ip);
            return None;
        }

        let source_node = source_node.unwrap();
        let dest_node = dest_node.unwrap();
        let next_hop_node = find_next_hop_node(state, source_node, dest_node);

        Some((
            source_node.clone(),
            dest_node.clone(),
            next_hop_node.clone(),
        ))
    });

    if nodes.is_none() {
        return;
    }

    let (source_node, dest_node, next_hop_node) = nodes.unwrap();

    log::debug!(
        "forwarding packet from {} to {} via next hop {}",
        source_node.name,
        dest_node.name,
        next_hop_node.name
    );
    send_packet_to_next_hop(tx, next_hop_node, interface, eth);
}

fn is_in_local_net(dest_ip: Ipv4Addr, interface: &NetworkInterface) -> bool {
    interface
        .ips
        .iter()
        .any(|i| i.contains(IpAddr::V4(dest_ip)))
}

/// Returns the next node along the chain.
/// This will return a node which is one stop closer to the destination along the chain.
/// The chain is defined by the order at which the appear in the Vec<Node>
fn find_next_hop_node<'a>(
    state: &'a State,
    source_node: &'a state::Node,
    dest_node: &'a state::Node,
) -> &'a Node {
    if source_node == dest_node {
        log::debug!("source node is equal to dest node, looping back");
        return dest_node;
    }

    let source_index = state
        .nodes
        .iter()
        .position(|c| c == source_node)
        .unwrap();
    let dest_index = state.nodes.iter().position(|c| c == dest_node).unwrap();

    let next_hop_index = if source_index < dest_index {
        source_index + 1
    } else {
        source_index - 1
    };

    return &state.nodes[next_hop_index];
}

fn send_packet_to_next_hop(
    tx: &mut Sender<Event>,
    next_hop: Node,
    interface: &NetworkInterface,
    eth: EthernetPacket,
) {
    if interface.mac.is_none() {
        log::warn!(
            "could not forward packet on interface {} as mac is not known",
            interface.name
        );
        return;
    }

    if next_hop.mac.is_none() {
        log::warn!(
            "could not forward packet to next hop {} as mac is not known",
            next_hop.name
        );
        return;
    }

    let mut new_eth = MutableEthernetPacket::owned(eth.packet().to_vec()).unwrap();

    new_eth.set_source(interface.mac.unwrap());
    new_eth.set_destination(next_hop.mac.unwrap());

    if let Err(err) = tx.send(Event::SendPacket(new_eth.consume_to_immutable())) {
        log::warn!("error while forwarding packet: {}", err);
    }
}
