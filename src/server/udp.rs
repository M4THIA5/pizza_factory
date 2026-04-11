use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::protocol::PeerInfo;
use crate::server::now_ms;
use crate::{protocol::{GossipMessage, Version}, server::GossipState};

pub fn start_udp_listener(state: Arc<GossipState>) -> Arc<UdpSocket> {
    let addr = state.own_addr;
    let socket = Arc::new(UdpSocket::bind(addr).expect("UDP bind échoué"));
    println!("[UDP] En écoute sur {addr}");

    let socket_listener = Arc::clone(&socket);
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            let (len, src) = match socket_listener.recv_from(&mut buf) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[UDP] recv_from : {e}");
                    continue;
                }
            };

            let src_str = src.to_string();

            match GossipMessage::decode(&buf[..len]) {
                Ok(GossipMessage::Ping(payload)) => {
                    let v = payload.body_version().map(|x| x.counter).unwrap_or(1);
                    println!("[UDP] Ping from {src_str} (counter={v})");

                    // Répond avec un Pong
                    let pong = GossipMessage::Pong(state.make_ping_pong_payload()).encode();
                    if let Err(e) = socket_listener.send_to(&pong, src) {
                        eprintln!("[UDP] Pong error to {src_str} : {e}");
                    }
                }
                Ok(GossipMessage::Pong(payload)) => {
                    let v = payload.body_version().map(|x| x.counter).unwrap_or(1);
                    println!("[UDP] Pong from {src_str} (counter={v})");
                }
                Ok(GossipMessage::Announce(payload)) => {
                    let version = payload.version().unwrap_or(Version { counter: 1, generation: now_ms() });
                    let counter = version.counter;
                    let capabilities = payload.capabilities().unwrap_or_default();
                    let recipes = payload.recipes().unwrap_or_default();

                    println!("[UDP] Announce from {src_str} (v{counter})");
                    if state.update_peer(src, PeerInfo { version, capabilities, recipes }) {
                        let announce = GossipMessage::Announce(state.make_announce_payload()).encode();
                        for peer_addr in state.peer_addrs() {
                            if peer_addr == state.own_addr {
                                continue;
                            }
                            if let Err(e) = socket_listener.send_to(&announce, peer_addr) {
                                eprintln!("[UDP] Announce (refresh) to {peer_addr} : {e}");
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[UDP] Decode failed from {src_str} : {e}");
                }
            }
        }
    });

    socket
}

pub fn start_gossip_emitter(state: Arc<GossipState>, initial_peers: Vec<SocketAddr>, interval: Duration, socket: Arc<UdpSocket>) {
    // Ajoute les pairs initiaux dans l'état
    {
        let mut peers = state.peers.lock().unwrap();
        for p in &initial_peers {
            peers.entry(*p).or_insert(PeerInfo {
                version: Version { counter: 1, generation: now_ms() },
                capabilities: vec![],
                recipes: vec![],
            });
        }
    }

    thread::spawn(move || {
        // Envoie une annonce initiale à tous les pairs connus
        let announce = GossipMessage::Announce(state.make_announce_payload()).encode();
        for peer_addr in &state.peer_addrs() {
            socket.send_to(&announce, peer_addr).ok();
        }

        loop {
            thread::sleep(interval);

            let targets = state.peer_addrs();
            if targets.is_empty() {
                continue;
            }

            let ping = GossipMessage::Ping(state.make_ping_pong_payload()).encode();

            for peer_addr in &targets {
                if *peer_addr == state.own_addr {
                    continue;
                }
                match socket.send_to(&ping, peer_addr) {
                    Ok(_) => println!("[Gossip] Ping to {peer_addr}"),
                    Err(e) => eprintln!("[Gossip] Ping error to {peer_addr} : {e}"),
                }
            }
        }
    });
}