use serde::{Deserialize, Serialize};

/// Version d'un nœud dans le réseau gossip.
/// counter : incrémenté à chaque changement d'état
/// generation : timestamp de démarrage (pour distinguer les redémarrages)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Version {
    pub counter: u64,
    pub generation: u64,
}

/// Payload commun aux messages Ping et Pong.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipPayload {
    /// Timestamp (ms depuis epoch) du dernier contact connu
    pub last_seen: u64,
    pub version: Version,
}

/// Messages UDP du protocole gossip.
/// Sérialisé en CBOR comme : ["Ping", { last_seen: u64, version: { counter: u64, generation: u64 } }]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "0", content = "1")]
pub enum GossipMessage {
    Ping(GossipPayload),
    Pong(GossipPayload),
}

impl GossipMessage {
    /// Sérialise le message en CBOR.
    /// Format observé dans le pcap : tableau à 2 éléments [type_string, payload_map]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let tuple: (&str, &GossipPayload) = match self {
            GossipMessage::Ping(p) => ("Ping", p),
            GossipMessage::Pong(p) => ("Pong", p),
        };
        ciborium::into_writer(&tuple, &mut buf).expect("CBOR encode failed");
        buf
    }

    /// Désérialise un message depuis des octets CBOR.
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        // Le message est un tableau [type_string, payload_map]
        let (kind, payload): (String, GossipPayload) =
            ciborium::from_reader(bytes).map_err(|e| format!("CBOR decode: {e}"))?;

        match kind.as_str() {
            "Ping" => Ok(GossipMessage::Ping(payload)),
            "Pong" => Ok(GossipMessage::Pong(payload)),
            other => Err(format!("Type inconnu : {other}")),
        }
    }
}