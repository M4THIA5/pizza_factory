use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;

use crate::server::GossipState;

// REPL = Read-Eval-Print Loop
pub fn run_repl(state: std::sync::Arc<GossipState>) {
    println!("\nCommandes : peers | recipes | capabilities | quit\n");
    let stdin = std::io::stdin();
    let mut line = String::new();

    loop {
        line.clear();
        if stdin.read_line(&mut line).is_err() {
            break;
        }
        match line.trim() {
            "peers" => {
                let peers = state.peers.lock().unwrap();
                if peers.is_empty() {
                    println!("Aucun pair connu.");
                } else {
                    for (addr, version) in peers.iter() {
                        println!("  {addr}  counter={} generation={}", version.counter, version.generation);
                    }
                }
            }
            "recipes" => {
                if state.recipes.is_empty() {
                    println!("Aucune recette chargée.");
                } else {
                    for name in state.recipes.keys() {
                        println!("  - {name}");
                    }
                }
            }
            "capabilities" => {
                if state.capabilities.is_empty() {
                    println!("Aucune capacité déclarée.");
                } else {
                    for cap in &state.capabilities {
                        println!("  - {cap}");
                    }
                }
            }
            "quit" | "exit" => std::process::exit(0),
            _ => println!("Commandes : peers | recipes | capabilities | quit"),
        }
        println!("\nCommandes : peers | recipes | capabilities | quit\n");
    }
}

pub fn start_tcp_server(state: Arc<GossipState>) {
    let addr = state.own_addr.clone();
    thread::spawn(move || {
        let listener = TcpListener::bind(&addr).expect("TCP bind échoué");
        println!("[TCP] Serveur en écoute sur {addr}");

        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let state = Arc::clone(&state);
                    thread::spawn(move || handle_tcp_connection(s, state));
                }
                Err(e) => eprintln!("[TCP] Erreur accept : {e}"),
            }
        }
    });
}

fn handle_tcp_connection(mut stream: TcpStream, _state: Arc<GossipState>) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    let mut buf = [0u8; 4096];

    match stream.read(&mut buf) {
        Ok(0) => {}
        Ok(len) => {
            let msg = std::str::from_utf8(&buf[..len])
                .unwrap_or("<binaire>")
                .trim();
            println!("[TCP] Message de {peer} : {msg}");
            let reply = format!("ACK : {msg}");
            if let Err(e) = stream.write_all(reply.as_bytes()) {
                eprintln!("[TCP] Erreur write : {e}");
            }
        }
        Err(e) => eprintln!("[TCP] Erreur read depuis {peer} : {e}"),
    }
}