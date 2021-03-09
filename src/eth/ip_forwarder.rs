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
use state::{Client, State};

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

    if source_mac == interface.mac.unwrap() {
        log::trace!("packet is sent from interface, ignoring");
        return;
    }

    log::trace!(
        "received packet from {} destined for {} in lan",
        source_mac,
        dest_ip
    );

    let clients = state.get(|state| {
        let source_client = state.clients.iter().find(|i| i.mac == Some(source_mac));

        if source_client.is_none() {
            log::trace!("could not find client with mac {}", source_mac);
            return None;
        }

        let dest_client = state.clients.iter().find(|i| i.ip == dest_ip);

        if dest_client.is_none() {
            log::debug!("could not find dest client with ip {}", dest_ip);
            return None;
        }

        let source_client = source_client.unwrap();
        let dest_client = dest_client.unwrap();
        let next_hop_client = find_next_hop_client(state, source_client, dest_client);

        Some((
            source_client.clone(),
            dest_client.clone(),
            next_hop_client.clone(),
        ))
    });

    if clients.is_none() {
        return;
    }

    let (source_client, dest_client, next_hop_client) = clients.unwrap();

    log::debug!(
        "forwarding packet from {} to {} via next hop {}",
        source_client.name,
        dest_client.name,
        next_hop_client.name
    );
    send_packet_to_next_hop(tx, next_hop_client, interface, eth);
}

fn is_in_local_net(dest_ip: Ipv4Addr, interface: &NetworkInterface) -> bool {
    interface
        .ips
        .iter()
        .any(|i| i.contains(IpAddr::V4(dest_ip)))
}

/// Returns the next client along the chain.
/// This will return a client which is one stop closer to the destination along the chain.
/// The chain is defined by the order at which the appear in the Vec<Client>
fn find_next_hop_client<'a>(
    state: &'a State,
    source_client: &'a state::Client,
    dest_client: &'a state::Client,
) -> &'a Client {
    if source_client == dest_client {
        log::debug!("source client is equal to dest client, looping back");
        return dest_client;
    }

    let source_index = state
        .clients
        .iter()
        .position(|c| c == source_client)
        .unwrap();
    let dest_index = state.clients.iter().position(|c| c == dest_client).unwrap();

    let next_hop_index = if source_index < dest_index {
        source_index + 1
    } else {
        source_index - 1
    };

    return &state.clients[next_hop_index];
}

fn send_packet_to_next_hop(
    tx: &mut Sender<Event>,
    next_hop: Client,
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
