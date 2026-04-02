use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::{protocol::{GossipMessage, Version}, server::GossipState};

/// Lance le thread d'écoute UDP : reçoit Ping → répond Pong, reçoit Pong → met à jour les pairs.
pub fn start_udp_listener(state: Arc<GossipState>) {
    let addr = state.own_addr.clone();
    thread::spawn(move || {
        let socket = UdpSocket::bind(&addr).expect("UDP bind échoué");
        println!("[UDP] En écoute sur {addr}");

        let mut buf = [0u8; 4096];
        loop {
            let (len, src) = match socket.recv_from(&mut buf) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[UDP] recv_from : {e}");
                    continue;
                }
            };

            let src_str = src.to_string();

            match GossipMessage::decode(&buf[..len]) {
                Ok(GossipMessage::Ping(payload)) => {
                    println!("[UDP] Ping de {src_str} (v{})", payload.version.counter);
                    state.update_peer(&src_str, payload.version);

                    // Répondre avec un Pong
                    let pong = GossipMessage::Pong(state.make_payload()).encode();
                    if let Err(e) = socket.send_to(&pong, src) {
                        eprintln!("[UDP] Pong vers {src_str} : {e}");
                    } else {
                        println!("[UDP] Pong → {src_str}");
                    }
                }
                Ok(GossipMessage::Pong(payload)) => {
                    println!("[UDP] Pong de {src_str} (v{})", payload.version.counter);
                    state.update_peer(&src_str, payload.version);
                }
                Err(e) => {
                    eprintln!("[UDP] Decode échoué depuis {src_str} : {e}");
                }
            }
        }
    });
}

/// Lance le thread gossip : envoie un Ping UDP à tous les pairs connus toutes les `interval`.
pub fn start_gossip_emitter(state: Arc<GossipState>, initial_peers: Vec<String>, interval: Duration) {
    thread::spawn(move || {
        // Socket d'émission sur port aléatoire
        let socket = UdpSocket::bind("127.0.0.1:0").expect("Gossip socket bind échoué");

        // Ajoute les pairs initiaux dans l'état
        {
            let mut peers = state.peers.lock().unwrap();
            for p in &initial_peers {
                peers.entry(p.clone()).or_insert(Version {
                    counter: 0,
                    generation: 0,
                });
            }
        }

        loop {
            thread::sleep(interval);

            let targets = state.peer_addrs();
            if targets.is_empty() {
                continue;
            }

            let ping = GossipMessage::Ping(state.make_payload()).encode();

            for peer_addr in &targets {
                if *peer_addr == state.own_addr {
                    continue;
                }
                match socket.send_to(&ping, peer_addr) {
                    Ok(_) => println!("[Gossip] Ping → {peer_addr}"),
                    Err(e) => eprintln!("[Gossip] Ping vers {peer_addr} : {e}"),
                }
            }
        }
    });
}