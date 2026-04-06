// pub mod utilities {
//     use ciborium::value::Value;
//     use std::net::SocketAddr;

//     pub fn get_tag_from_addr(addr: SocketAddr) -> Value {
//         Value::Tag(260, Box::new(Value::Text(addr.to_string())))
//     }

//     pub fn text(s: impl Into<String>) -> Value {
//         Value::Text(s.into())
//     }

//     pub fn map(values: Vec<(Value, Value)>) -> Value {
//         Value::Map(values)
//     }

//     pub fn array_text(list: &[String]) -> Value {
//         Value::Array(list.iter().cloned().map(Value::Text).collect())
//     }

//     pub fn array_addr(peers: &[SocketAddr]) -> Value {
//         Value::Array(peers.iter().copied().map(get_tag_from_addr).collect())
//     }
// }