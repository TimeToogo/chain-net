use std::ascii::escape_default;

use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;
use pnet::packet::udp::UdpPacket;
use pnet::packet::Packet;

use crate::args::Args;

pub fn dump_packet(args: &Args, packet: &Ipv4Packet) -> Result<(), String> {
    if args.dump == 0 {
        return Ok(());
    }

    println!("\n ------ packet received ------ ");
    println!("IP   | src ip: {} | dst ip: {} |", packet.get_source(), packet.get_destination());

    if args.dump == 1 {
        return Ok(());
    }

    let payload = match packet.get_next_level_protocol() {
        IpNextHeaderProtocols::Tcp => {
            dump_tcp_header(TcpPacket::new(packet.payload()).ok_or("invalid tcp packet")?)
        }
        IpNextHeaderProtocols::Udp => {
            dump_udp_header(UdpPacket::new(packet.payload()).ok_or("invalid udp packet")?)
        }
        IpNextHeaderProtocols::Icmp => {
            dump_icmp_header(IcmpPacket::new(packet.payload()).ok_or("invalid icmp packet")?)
        }
        _ => {
            println!("Unknown transport protocol");
            return Ok(());
        }
    };

    if args.dump == 2 {
        return Ok(());
    }

    dump_app_payload(payload.as_slice());
    Ok(())
}

fn dump_tcp_header<'a>(packet: TcpPacket<'a>) -> Vec<u8> {
    println!(
        "TCP  | src port: {} | dest port: {} |",
        packet.get_source(),
        packet.get_destination()
    );
    packet.payload().to_vec()
}

fn dump_udp_header<'a>(packet: UdpPacket<'a>) -> Vec<u8> {
    println!(
        "UDP  | src port: {} | dest port: {} |",
        packet.get_source(),
        packet.get_destination()
    );
    packet.payload().to_vec()
}

fn dump_icmp_header<'a>(packet: IcmpPacket<'a>) -> Vec<u8> {
    let icmp_type = match packet.get_icmp_type() {
        IcmpTypes::EchoRequest => "Echo Request",
        IcmpTypes::EchoReply => "Echo Reply",
        IcmpTypes::InformationRequest => "Information Request",
        IcmpTypes::DestinationUnreachable => "Destination Unreachable",
        IcmpTypes::Traceroute => "Traceroute",
        _ => "Other",
    };
    println!("ICMP | type: {} |", icmp_type);
    packet.payload().to_vec()
}

fn dump_app_payload(payload: &[u8]) {
    let escaped = String::from_utf8(
        payload
            .iter()
            .map(|i| escape_default(*i))
            .flatten()
            .collect(),
    ).unwrap();

    println!("{}", escaped);
}
