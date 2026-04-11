//! Trames TCP communes : préfixe u32 BE + CBOR.

pub mod framing;

use ciborium::Value;

pub fn message_key(v: &Value) -> Option<&str> {
    match v {
        Value::Map(m) => m.first().and_then(|(k, _)| match k {
            Value::Text(s) => Some(s.as_str()),
            _ => None,
        }),
        _ => None,
    }
}

pub fn map_get<'a>(m: &'a [(Value, Value)], key: &str) -> Option<&'a Value> {
    m.iter()
        .find(|(k, _)| k == &Value::Text(key.into()))
        .map(|(_, v)| v)
}
