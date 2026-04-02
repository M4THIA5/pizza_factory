use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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
    pub own_addr: String,
    /// Pairs connus : adresse -> dernière version reçue
    pub peers: Mutex<HashMap<String, Version>>,
    /// Version locale du nœud (incrémentée à chaque Ping émis)
    pub version: Mutex<Version>,
    /// Recettes disponibles sur ce nœud
    pub recipes: HashMap<String, Recipe>,
    /// Capabilités de ce nœud (noms d'actions supportées)
    pub capabilities: Vec<String>,
}

impl GossipState {
    pub fn new(own_addr: String, generation: u64, recipes: HashMap<String, Recipe>, capabilities: Vec<String>) -> Arc<Self> {
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

    /// Incrémente le counter et retourne la version courante.
    fn bump_version(&self) -> Version {
        let mut v = self.version.lock().unwrap();
        v.counter += 1;
        v.clone()
    }

    /// Construit un payload gossip avec la version courante.
    fn make_payload(&self) -> GossipPayload {
        GossipPayload {
            last_seen: now_ms(),
            version: self.bump_version(),
        }
    }

    /// Enregistre ou met à jour un pair.
    fn update_peer(&self, addr: &str, version: Version) {
        let mut peers = self.peers.lock().unwrap();
        peers.insert(addr.to_string(), version);
    }

    /// Retourne la liste des adresses des pairs connus.
    pub fn peer_addrs(&self) -> Vec<String> {
        self.peers.lock().unwrap().keys().cloned().collect()
    }
}

pub fn run_server(addr: String, initial_peers: Vec<String>, capabilities: Vec<String>, recipes_path: String, gossip_interval: u64) {
    let generation = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    println!("Agent démarré : {addr} (generation={generation})");

    let recipes = load_recipes(&recipes_path).unwrap_or_else(|e| {
        eprintln!("Recettes non chargées : {e}");
        HashMap::new()
    });

    let state = GossipState::new(addr.clone(), generation, recipes, capabilities);
    udp::start_udp_listener(state.clone());
    udp::start_gossip_emitter(state.clone(), initial_peers, Duration::from_secs(gossip_interval));
    tcp::start_tcp_server(state.clone());
    tcp::run_repl(state);
}