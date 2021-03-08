mod event;
mod arp;

use std::{sync::mpsc, thread, time::Duration};

use anyhow::anyhow;
use anyhow::{bail, Result};
use event::Event;
use pnet::datalink::{self, Channel, DataLinkReceiver};
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::{datalink::DataLinkSender, packet::Packet};

use crate::{args::Args, state::SharedState};

pub fn start(args: Args, mut state: SharedState) -> Result<()> {
    log::info!(
        "starting ethernet forwarder on interface {}",
        args.interface
    );

    let interface = datalink::interfaces()
        .into_iter()
        .filter(|i| i.name == args.interface)
        .next()
        .ok_or(anyhow!("could not find interface named {}", args.interface))?;

    let (mut dtx, drx) = match datalink::channel(&interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => bail!("Unknown channel type"),
        Err(err) => bail!("Error while creating raw socket: {}", err),
    };

    let (tx, rx) = mpsc::channel::<Event>();

    spawn(&tx, &state, move |tx, _| receive_packets(drx, tx));
    spawn(&tx, &state, |tx, state| terminate_if_stopped(state, tx));
    spawn(&tx, &state, move |tx, state| arp::send_requests(state, interface.clone(), tx));

    loop {
        match rx.recv()? {
            Event::PacketReceived(packet) => process_packet(&mut state, packet),
            Event::SendPacket(packet) => send_packet(&mut dtx, packet),
            Event::Terminate(res) => break res?,
        }
    }

    log::info!("ethernet fowarder shutting down");

    Ok(())
}

fn spawn<F>(tx: &mpsc::Sender<Event>, state: &SharedState, f: F)
where
    F: FnOnce(mpsc::Sender<Event>, SharedState) -> () + Send + 'static,
{
    let tx = tx.clone();
    let state = state.clone();

    thread::spawn(move || f(tx, state));
}

fn send_packet(dtx: &mut Box<dyn DataLinkSender>, packet: EthernetPacket) {
    dtx.send_to(packet.packet(), None);
}

fn receive_packets(mut drx: Box<dyn DataLinkReceiver>, tx: mpsc::Sender<Event>) {
    loop {
        match drx.next() {
            Ok(packet) => {
                let packet = match EthernetPacket::owned(packet.to_vec()) {
                    Some(p) => p,
                    None => {
                        log::warn!("failed to parse ethernet packet");
                        continue;
                    }
                };

                log::trace!("packet received from {}", packet.get_source().to_string());
                tx.send(Event::PacketReceived(packet)).unwrap();
            }
            Err(err) => {
                tx.send(Event::Terminate(Err(anyhow!(
                    "error while receiving packets: {}",
                    err
                ))))
                .unwrap();
                return;
            }
        }
    }
}

fn process_packet(state: &mut SharedState, packet: EthernetPacket) {
    match packet.get_ethertype() {
        EtherTypes::Arp => arp::process_packet(state, packet),
        _ => {}
    }
}

fn terminate_if_stopped(state: SharedState, tx: mpsc::Sender<Event>) {
    while state.running() {
        thread::sleep(Duration::from_millis(500));
    }

    tx.send(Event::Terminate(Ok(()))).unwrap();
}
