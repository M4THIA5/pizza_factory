//! Client TCP : trames `u32` big-endian + CBOR (comme `pizza_factory client`).

use std::net::SocketAddr;

use ciborium::Value;

use crate::wire::framing::{read_frame, write_frame};
use crate::wire::{map_get, message_key};

/// CBOR → JSON pour affichage proche de `pizza_factory` (`@@TAGGED@@` pour les tags).
pub fn value_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Integer(i) => {
            let n = i128::from(*i);
            if let Ok(u) = u64::try_from(n) {
                serde_json::Value::Number(u.into())
            } else if let Ok(i) = i64::try_from(n) {
                serde_json::Value::Number(i.into())
            } else {
                serde_json::Value::String(n.to_string())
            }
        }
        Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Value::Bytes(b) => serde_json::Value::String(String::from_utf8_lossy(b).into_owned()),
        Value::Text(s) => serde_json::Value::String(s.clone()),
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Null => serde_json::Value::Null,
        Value::Tag(t, inner) => serde_json::json!({
            "@@TAGGED@@": [t, value_to_json(inner)]
        }),
        Value::Array(a) => serde_json::Value::Array(a.iter().map(value_to_json).collect()),
        Value::Map(m) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in m {
                if let Value::Text(key) = k {
                    obj.insert(key.clone(), value_to_json(v));
                }
            }
            serde_json::Value::Object(obj)
        }
        _ => serde_json::Value::String(format!("{v:?}")),
    }
}

fn order_id_to_string(v: &Value) -> Option<String> {
    match v {
        Value::Tag(37, inner) => match inner.as_ref() {
            Value::Text(s) => Some(s.clone()),
            Value::Bytes(b) => String::from_utf8(b.clone()).ok(),
            _ => None,
        },
        Value::Text(s) => Some(s.clone()),
        _ => None,
    }
}

/// Si `completed_order.result` est une chaîne JSON, la pretty-print ; sinon `None`.
fn pretty_completed_order(msg: &Value) -> Option<String> {
    let m = match msg {
        Value::Map(m) => m.as_slice(),
        _ => return None,
    };
    let inner = match map_get(m, "completed_order")? {
        Value::Map(im) => im.as_slice(),
        _ => return None,
    };
    let result = map_get(inner, "result")?;
    let s = match result {
        Value::Text(t) => t.as_str(),
        _ => return None,
    };
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    serde_json::to_string_pretty(&v).ok()
}

fn extract_order_receipt_id(msg: &Value) -> Option<String> {
    let m = match msg {
        Value::Map(m) => m.as_slice(),
        _ => return None,
    };
    let inner = match map_get(m, "order_receipt")? {
        Value::Map(im) => im.as_slice(),
        _ => return None,
    };
    let id = map_get(inner, "order_id")?;
    order_id_to_string(id)
}

pub fn build_order(recipe: &str) -> Value {
    Value::Map(vec![(
        Value::Text("order".into()),
        Value::Map(vec![(
            Value::Text("recipe_name".into()),
            Value::Text(recipe.into()),
        )]),
    )])
}

pub fn build_list_recipes() -> Value {
    Value::Map(vec![(
        Value::Text("list_recipes".into()),
        Value::Map(vec![]),
    )])
}

pub fn build_get_recipe(recipe: &str) -> Value {
    Value::Map(vec![(
        Value::Text("get_recipe".into()),
        Value::Map(vec![(
            Value::Text("recipe_name".into()),
            Value::Text(recipe.into()),
        )]),
    )])
}

/// `cargo run -- client --peer HOST:PORT order RECIPE`
pub fn run_order(peer: SocketAddr, recipe: &str) -> Result<(), String> {
    println!("Ordering recipe recipe={recipe} peer={peer}");
    let mut stream =
        std::net::TcpStream::connect(peer).map_err(|e| format!("connexion TCP : {e}"))?;

    write_frame(&mut stream, &build_order(recipe)).map_err(|e| e.to_string())?;

    loop {
        let msg = read_frame(&mut stream).map_err(|e| e.to_string())?;
        match message_key(&msg).unwrap_or("") {
            "order_receipt" => {
                let id = extract_order_receipt_id(&msg).unwrap_or_else(|| "?".into());
                println!("Order receipt {id}; waiting for completion...");
            }
            "process_payload" => {}
            "deliver" => {
                println!("Order completed successfully");
                println!("Recipe {recipe}:");
                println!(
                    "{}",
                    serde_json::to_string_pretty(&value_to_json(&msg))
                        .map_err(|e| e.to_string())?
                );
                return Ok(());
            }
            "completed_order" => {
                println!("Order completed successfully");
                println!("Recipe {recipe}:");
                if let Some(pretty) = pretty_completed_order(&msg) {
                    println!("{pretty}");
                } else {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&value_to_json(&msg))
                            .map_err(|e| e.to_string())?
                    );
                }
                return Ok(());
            }
            "order_declined" | "failed_order" | "production_error" => {
                return Err(format!(
                    "commande refusée ou erreur : {}",
                    serde_json::to_string_pretty(&value_to_json(&msg)).unwrap_or_default()
                ));
            }
            other => {
                return Err(format!(
                    "réponse inattendue `{other}` : {}",
                    serde_json::to_string_pretty(&value_to_json(&msg)).unwrap_or_default()
                ));
            }
        }
    }
}

pub fn run_list_recipes(peer: SocketAddr) -> Result<(), String> {
    let mut stream =
        std::net::TcpStream::connect(peer).map_err(|e| format!("connexion TCP : {e}"))?;
    write_frame(&mut stream, &build_list_recipes()).map_err(|e| e.to_string())?;
    let msg = read_frame(&mut stream).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value_to_json(&msg)).map_err(|e| e.to_string())?
    );
    Ok(())
}

pub fn run_get_recipe(peer: SocketAddr, recipe: &str) -> Result<(), String> {
    let mut stream =
        std::net::TcpStream::connect(peer).map_err(|e| format!("connexion TCP : {e}"))?;
    write_frame(&mut stream, &build_get_recipe(recipe)).map_err(|e| e.to_string())?;
    let msg = read_frame(&mut stream).map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&value_to_json(&msg)).map_err(|e| e.to_string())?
    );
    Ok(())
}
