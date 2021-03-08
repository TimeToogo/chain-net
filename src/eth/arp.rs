use std::{net::{IpAddr, Ipv4Addr}, sync::mpsc::Sender, thread, time::Duration};

use pnet::packet::arp::{ArpOperations, ArpPacket, MutableArpPacket, ArpHardwareTypes};
use pnet::{
    packet::Packet,
    datalink::NetworkInterface,
    packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket},
    util::MacAddr,
};

use crate::state::SharedState;

use super::event::Event;

pub fn send_requests(state: SharedState, interface: NetworkInterface, mut tx: Sender<Event>) {
    loop {
        // Get clients without mac addr
        let clients = state
            .inner_clone()
            .clients
            .into_iter()
            .filter(|i| i.mac.is_none())
            .collect::<Vec<_>>();

        for client in clients {
            log::info!("sending arp request for ip {}", client.ip.to_string());
            send_arp_request(&interface, client.ip, &mut tx);
        }

        thread::sleep(Duration::from_millis(1000));
    }
}

fn send_arp_request(interface: &NetworkInterface, ip: Ipv4Addr, tx: &mut Sender<Event>) -> () {
    let source_mac = interface.mac.expect("failed to get mac from interface");
    let source_ip = interface
        .ips
        .iter()
        .find(|i| i.is_ipv4())
        .map(|i| i.ip())
        .map(|i| match i {
            IpAddr::V4(i) => i,
            _ => unreachable!(),
        })
        .expect("failed to get ipv4 address of interface");

    let mut buff = [0u8; 42]; // 14 (eth frame header) + 28 (arp request length)
    let (eth_buff, arp_buff) = buff.split_at_mut(14);

    let mut eth = MutableEthernetPacket::new(eth_buff).unwrap();
    eth.set_source(source_mac);
    eth.set_destination(MacAddr::broadcast());
    eth.set_ethertype(EtherTypes::Arp);

    let mut arp = MutableArpPacket::new(arp_buff).unwrap();
    arp.set_hardware_type(ArpHardwareTypes::Ethernet);
    arp.set_protocol_type(EtherTypes::Ipv4);
    arp.set_hw_addr_len(6);
    arp.set_proto_addr_len(4);
    arp.set_operation(ArpOperations::Request);
    arp.set_sender_hw_addr(source_mac);
    arp.set_sender_proto_addr(source_ip);
    arp.set_target_hw_addr(MacAddr::zero());
    arp.set_target_proto_addr(ip);

    tx.send(Event::SendPacket(EthernetPacket::owned(buff.to_vec()).unwrap())).unwrap();
}

pub fn process_packet(state: &mut SharedState, eth: EthernetPacket) {
    let arp = ArpPacket::new(eth.payload()).unwrap();
    log::trace!("packet is arp");

    if arp.get_operation() != ArpOperations::Reply {
        log::trace!("arp packet is not response");
        return;
    }

    if arp.get_protocol_type() != EtherTypes::Ipv4 {
        log::trace!("protocol type is not ipv4");
        return;
    }

    let sender_mac = arp.get_sender_hw_addr();
    let sender_ip = arp.get_sender_proto_addr();

    log::info!("received arp response for ip {} with mac {}", sender_ip, sender_mac);
    
    state.update(|state| {
        let client = state.clients.iter_mut()
            .find(|i| i.ip == sender_ip);

        if client.is_none() {
            log::warn!("could not find client with ip {}", sender_ip);
            return;
        }

        client.unwrap().mac = Some(sender_mac);
    });
}
