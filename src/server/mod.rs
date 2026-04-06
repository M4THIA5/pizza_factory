use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ciborium::Value;

use crate::protocol::{GossipPayload, Version};
use crate::recipe::{Recipe, load_recipes};

pub mod udp;
pub mod tcp;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub struct GossipState {
    /// Adresse de ce nœud
    pub own_addr: SocketAddr,
    /// Pairs connus : adresse -> dernière version reçue
    pub peers: Mutex<HashMap<SocketAddr, Version>>,
    /// Version locale du nœud (incrémentée à chaque Ping émis)
    pub version: Mutex<Version>,
    /// Recettes disponibles sur ce nœud
    pub recipes: HashMap<String, Recipe>,
    /// Capabilités de ce nœud (noms d'actions supportées)
    pub capabilities: Vec<String>,
}

impl GossipState {
    pub fn new(own_addr: SocketAddr, generation: u64, recipes: HashMap<String, Recipe>, capabilities: Vec<String>) -> Arc<Self> {
        Arc::new(Self {
            own_addr,
            peers: Mutex::new(HashMap::new()),
            version: Mutex::new(Version {
                counter: 0,
                generation,
            }),
            recipes,
            capabilities,
        })
    }

    /// Incrémente le counter de la version.
    fn bump_version(&self){
        let mut v = self.version.lock().unwrap();
        v.counter += 1;
    }

    /// Construit le payload d'un Announce.
    pub fn make_announce_payload(&self) -> GossipPayload {
        let version = self.version.lock().unwrap().clone();

        GossipPayload(Value::Map(vec![
            (
                Value::Text("Announce".to_string()),
                Value::Map(vec![
                    (Value::Text("node_addr".to_string()), Value::Tag(260, Box::new(Value::Text(self.own_addr.to_string())))),
                    (Value::Text("capabilities".to_string()), Value::Array(self.capabilities.iter().cloned().map(Value::Text).collect())),
                    (Value::Text("recipes".to_string()), Value::Array(self.recipes.keys().cloned().map(Value::Text).collect())),
                    (Value::Text("peers".to_string()), Value::Array(self.peer_addrs().into_iter().map(|a| Value::Tag(260, Box::new(Value::Text(a.to_string())))).collect())),
                    (Value::Text("version".to_string()), Value::Map(vec![
                        (Value::Text("counter".to_string()), Value::Integer(version.counter.into())),
                        (Value::Text("generation".to_string()), Value::Integer(version.generation.into())),
                    ])),
                ]),
            )
        ]))
    }

    /// Construit le payload d'un Ping ou Pong.
    pub fn make_ping_pong_payload(&self) -> GossipPayload {
        let version = self.version.lock().unwrap().clone();

        GossipPayload(Value::Map(vec![
            (Value::Text("last_seen".to_string()), Value::Integer(now_ms().into())),
            (Value::Text("version".to_string()), Value::Map(vec![
                (Value::Text("counter".to_string()), Value::Integer(version.counter.into())),
                (Value::Text("generation".to_string()), Value::Integer(version.generation.into())),
            ])),
        ]))
    }

    /// Enregistre ou met à jour un pair.
    pub fn update_peer(&self, addr: SocketAddr, version: Version) {
        let mut peers = self.peers.lock().unwrap();
        let is_new = !peers.contains_key(&addr);
        peers.insert(addr, version);
        drop(peers);

        if is_new {
            self.bump_version();
        }
    }

    /// Retourne la liste des adresses des pairs connus.
    pub fn peer_addrs(&self) -> Vec<SocketAddr> {
        self.peers.lock().unwrap().keys().cloned().collect()
    }
}

pub fn run_server(addr: SocketAddr, initial_peers: Vec<SocketAddr>, capabilities: Vec<String>, recipes_path: String, gossip_interval: u64) {
    let generation = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    println!("Agent démarré : {addr} (generation={generation})");

    let recipes = load_recipes(&recipes_path).unwrap_or_else(|e| {
        eprintln!("Recettes non chargées : {e}");
        HashMap::new()
    });

    let state = GossipState::new(addr, generation, recipes, capabilities);
    let socket = udp::start_udp_listener(state.clone());

    udp::start_gossip_emitter(state.clone(), initial_peers, Duration::from_secs(gossip_interval), socket);
    
    tcp::start_tcp_server(state.clone());
    tcp::run_repl(state);
}