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

    fn extract_u64(map: &[(Value, Value)], key: &str) -> Option<u64> {
        map.iter().find_map(|(k, v)| {
            if k == &Value::Text(key.into()) {
                match v {
                    Value::Integer(i) => u64::try_from(*i).ok(),
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    pub fn counter(&self) -> Option<u64> {
        let announce = self.get_announce_map()?;
        let version = Self::get_version_map(announce)?;
        Self::extract_u64(version, "counter")
    }

    pub fn version(&self) -> Option<Version> {
        let announce = self.get_announce_map()?;
        let version_map = Self::get_version_map(announce)?;
        Some(Version {
            counter: Self::extract_u64(version_map, "counter")?,
            generation: Self::extract_u64(version_map, "generation")?,
        })
    }
}

/// Version d'un nœud dans le réseau gossip.
/// counter : incrémenté à chaque changement d'état
/// generation : timestamp de démarrage (pour distinguer les redémarrages)
#[derive(Debug, Clone, PartialEq)]
pub struct Version {
    pub counter: u64,
    pub generation: u64,
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