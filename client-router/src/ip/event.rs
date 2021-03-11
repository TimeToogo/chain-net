use anyhow::Result;
use pnet::packet::ethernet::EthernetPacket;

pub enum Event {
    PacketReceived(EthernetPacket<'static>),
    SendPacket(EthernetPacket<'static>),
    Terminate(Result<()>)
}