use ciborium::Value;

#[derive(Debug, Clone)]
pub struct GossipPayload(pub Value);

impl GossipPayload {
    fn get_announce_map(&self) -> Option<&Vec<(Value, Value)>> {
        let root = match &self.0 {
            Value::Map(m) => m,
            _ => return None,
        };

        root.iter().find_map(|(k, v)| {
            if k == &Value::Text("Announce".into()) {
                match v {
                    Value::Map(m) => Some(m),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    fn get_version_map(announce: &[(Value, Value)]) -> Option<&Vec<(Value, Value)>> {
        announce.iter().find_map(|(k, v)| {
            if k == &Value::Text("version".into()) {
                match v {
                    Value::Map(m) => Some(m),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    fn extract_u32(map: &[(Value, Value)], key: &str) -> Option<u32> {
        map.iter().find_map(|(k, v)| {
            if k == &Value::Text(key.into()) {
                match v {
                    Value::Integer(i) => u32::try_from(*i).ok(),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    pub fn version(&self) -> Option<Version> {
        let announce = self.get_announce_map()?;
        let version_map = Self::get_version_map(announce)?;
        Some(Version {
            counter: Self::extract_u32(version_map, "counter")?,
            generation: Self::extract_u32(version_map, "generation")?,
        })
    }

    /// Version dans le corps du message (Ping, Pong, ou map interne d'Announce).
    pub fn body_version(&self) -> Option<Version> {
        let root = match &self.0 {
            Value::Map(m) if !m.is_empty() => m,
            _ => return None,
        };
        let (_, body) = root.first()?;
        let body_map = match body {
            Value::Map(m) => m.as_slice(),
            _ => return None,
        };
        let version_map = Self::get_version_map(body_map)?;
        Some(Version {
            counter: Self::extract_u32(version_map, "counter")?,
            generation: Self::extract_u32(version_map, "generation")?,
        })
    }

    pub fn capabilities(&self) -> Option<Vec<String>> {
        let announce = self.get_announce_map()?;
        let caps = announce.iter().find_map(|(k, v)| {
            if k == &Value::Text("capabilities".into()) {
                match v {
                    Value::Array(items) => Some(items),
                    _ => None,
                }
            } else {
                None
            }
        })?;

        caps.iter().map(|v| match v {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        }).collect()
    }

    pub fn recipes(&self) -> Option<Vec<String>> {
        let announce = self.get_announce_map()?;
        let recs = announce.iter().find_map(|(k, v)| {
            if k == &Value::Text("recipes".into()) {
                match v {
                    Value::Array(items) => Some(items),
                    _ => None,
                }
            } else {
                None
            }
        })?;

        recs.iter().map(|v| match v {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        }).collect()
}
}

/// Version d'un nœud dans le réseau gossip.
/// counter : incrémenté à chaque changement d'état
/// generation : timestamp de démarrage (pour distinguer les redémarrages)
#[derive(Debug, Clone, PartialEq)]
pub struct Version {
    pub counter: u32,
    pub generation: u32,
}

pub struct PeerInfo {
    pub version: Version,
    pub capabilities: Vec<String>,
    pub recipes: Vec<String>,
}

/// Messages UDP du protocole gossip.
#[derive(Debug, Clone)]
pub enum GossipMessage {
    Ping(GossipPayload),
    Pong(GossipPayload),
    Announce(GossipPayload),
}

impl GossipMessage {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(512);
        let value = match self {
            GossipMessage::Ping(p) => Value::Map(vec![
                (Value::Text("Ping".into()), p.0.clone()),
            ]),
            GossipMessage::Pong(p) => Value::Map(vec![
                (Value::Text("Pong".into()), p.0.clone()),
            ]),
            GossipMessage::Announce(p) => p.0.clone(), // déjà {"Announce": ...}
        };
        ciborium::into_writer(&value, &mut buf).expect("CBOR encode failed");
        buf
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let value: Value =
            ciborium::de::from_reader(bytes).map_err(|e| format!("CBOR decode: {e}"))?;

        let map = match &value {
            Value::Map(m) => m,
            _ => return Err(format!("attendu une map, reçu : {value:?}")),
        };

        let (kind, _) = map.first().ok_or("map vide")?;

        match kind {
            Value::Text(s) if s == "Announce" => Ok(GossipMessage::Announce(GossipPayload(value))),
            Value::Text(s) if s == "Ping" => Ok(GossipMessage::Ping(GossipPayload(value))),
            Value::Text(s) if s == "Pong" => Ok(GossipMessage::Pong(GossipPayload(value))),
            other => Err(format!("Type inconnu : {other:?}")),
        }
    }
}