use std::{
    net::{IpAddr, Ipv4Addr},
    sync::mpsc::Sender,
};

use pnet::{
    datalink::NetworkInterface,
    packet::ethernet::{EthernetPacket, MutableEthernetPacket},
    packet::Packet,
};
use pnet::{ipnetwork::IpNetwork, packet::ipv4::Ipv4Packet};

use crate::args::Args;

use super::dumper::dump_packet;
use super::event::Event;

pub fn process_packet(
    args: &Args,
    tx: &mut Sender<Event>,
    eth: EthernetPacket,
    interface: &NetworkInterface,
) {
    let ip = Ipv4Packet::new(eth.payload()).unwrap();
    log::trace!("packet is ipv4");

    let src_mac = eth.get_source();

    if interface.mac == Some(src_mac) {
        log::trace!("packet is from target interface, ignoring");
        return;
    }

    let src_ip = ip.get_source();
    let dest_ip = ip.get_destination();

    log::trace!("packet from {} to dest {}", src_ip, dest_ip);

    if !is_in_local_net(src_ip, interface) {
        log::trace!("packet src is not local network, ignoring");
        return;
    }

    if !is_in_local_net(dest_ip, interface) {
        log::trace!("packet dest is not local network, ignoring");
        return;
    }

    let host_is_dest = interface.ips.iter().any(|i| i.ip() == IpAddr::V4(dest_ip));
    if host_is_dest {
        log::trace!("received packet from {} to localhost", src_ip);
        let _ = dump_packet(args, &ip);
        return;
    }

    log::trace!("received packet from {} to {}", src_ip, dest_ip);

    if args.promisc {
        let _ = dump_packet(args, &ip);
    }

    // This packet is not for us, return to the sender
    return_to_sender(tx, interface, eth);
}

fn is_in_local_net(dest_ip: Ipv4Addr, interface: &NetworkInterface) -> bool {
    interface
        .ips
        .iter()
        .filter(|i| i.is_ipv4())
        // Some hackery if the subnet mask is set at /32 we
        // pretend it's /24
        // Having a /32 subnet mask on the central router stops
        // clients from routing the packets at L2 which can make
        // it for the central router to modify the routing of those packets.
        .map(|i| {
            if i.prefix() == 32 {
                IpNetwork::new(i.ip(), 24).unwrap()
            } else {
                i.clone()
            }
        })
        .any(|i| i.contains(IpAddr::V4(dest_ip)))
}

fn return_to_sender(tx: &mut Sender<Event>, interface: &NetworkInterface, eth: EthernetPacket) {
    if interface.mac.is_none() {
        log::warn!(
            "could not send packet on interface {} as mac is not known",
            interface.name
        );
        return;
    }

    let mut new_eth = MutableEthernetPacket::owned(eth.packet().to_vec()).unwrap();

    new_eth.set_source(interface.mac.unwrap());
    new_eth.set_destination(eth.get_source());

    log::trace!("routing packet back to sender");
    if let Err(err) = tx.send(Event::SendPacket(new_eth.consume_to_immutable())) {
        log::warn!("error while sending packet: {}", err);
    }
}
