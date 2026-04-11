use std::io;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use ciborium::Value;
use serde_json::json;
use uuid::Uuid;

use crate::recipe::{flatten_actions, recipe_to_formula, Recipe};
use crate::server::GossipState;
use crate::wire::framing::{read_frame, write_frame};
use crate::wire::{map_get, message_key};

fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64
}

fn as_map_slice(v: &Value) -> Option<&[(Value, Value)]> {
    match v {
        Value::Map(m) => Some(m.as_slice()),
        _ => None,
    }
}

fn parse_recipe_name_order(msg: &Value) -> Option<String> {
    let root = as_map_slice(msg)?;
    let order = map_get(root, "order")?;
    let inner = as_map_slice(order)?;
    map_get(inner, "recipe_name").and_then(|v| match v {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    })
}

fn parse_recipe_name_get(msg: &Value) -> Option<String> {
    let root = as_map_slice(msg)?;
    let g = map_get(root, "get_recipe")?;
    let inner = as_map_slice(g)?;
    map_get(inner, "recipe_name").and_then(|v| match v {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    })
}

fn content_line_for_action(name: &str) -> String {
    match name {
        "MakeDough" => "Dough: ready\n".into(),
        "AddOliveOil" => "Olive Oil drizzled\n".into(),
        other => format!("{other}: done\n"),
    }
}

fn order_id_value(uuid_str: &str) -> Value {
    Value::Tag(37, Box::new(Value::Text(uuid_str.into())))
}

fn order_receipt_frame(uuid_str: &str) -> Value {
    Value::Map(vec![(
        Value::Text("order_receipt".into()),
        Value::Map(vec![(
            Value::Text("order_id".into()),
            order_id_value(uuid_str),
        )]),
    )])
}

fn order_declined_frame(msg: &str) -> Value {
    Value::Map(vec![(
        Value::Text("order_declined".into()),
        Value::Map(vec![(
            Value::Text("message".into()),
            Value::Text(msg.into()),
        )]),
    )])
}

fn completed_order_frame(recipe_name: &str, result_json: &str) -> Value {
    Value::Map(vec![(
        Value::Text("completed_order".into()),
        Value::Map(vec![
            (
                Value::Text("recipe_name".into()),
                Value::Text(recipe_name.into()),
            ),
            (
                Value::Text("result".into()),
                Value::Text(result_json.into()),
            ),
        ]),
    )])
}

fn recipe_list_frame(state: &GossipState) -> Value {
    let mut names: Vec<String> = state.recipes.keys().cloned().collect();
    names.sort();
    let items: Vec<Value> = names
        .into_iter()
        .map(|name| {
            Value::Map(vec![(Value::Text("name".into()), Value::Text(name))])
        })
        .collect();
    Value::Map(vec![(
        Value::Text("recipe_list_answer".into()),
        Value::Map(vec![(
            Value::Text("recipes".into()),
            Value::Array(items),
        )]),
    )])
}

fn recipe_answer_frame(r: &Recipe) -> Value {
    let formula = recipe_to_formula(r);
    Value::Map(vec![(
        Value::Text("recipe_answer".into()),
        Value::Map(vec![(
            Value::Text("recipe".into()),
            Value::Map(vec![
                (Value::Text("name".into()), Value::Text(r.name.clone())),
                (Value::Text("formula".into()), Value::Text(formula)),
            ]),
        )]),
    )])
}

fn forward_to_json(target: std::net::SocketAddr, ts: u64) -> serde_json::Value {
    let mut to_tagged = serde_json::Map::new();
    to_tagged.insert(
        "@@TAGGED@@".to_string(),
        json!([260u64, target.to_string()]),
    );
    json!({
        "Forward": {
            "to": serde_json::Value::Object(to_tagged),
            "timestamp": ts
        }
    })
}

fn handle_order(
    stream: &mut TcpStream,
    state: &GossipState,
    msg: &Value,
    client_addr: &str,
) -> io::Result<()> {
    let Some(name) = parse_recipe_name_order(msg) else {
        println!("[TCP] refus commande depuis {client_addr} : CBOR « order » invalide");
        write_frame(stream, &order_declined_frame("invalid order"))?;
        return Ok(());
    };
    let Some(r) = state.recipes.get(&name) else {
        println!("[TCP] refus depuis {client_addr} : recette inconnue « {name} »");
        write_frame(
            stream,
            &order_declined_frame(&format!("unknown recipe: {name}")),
        )?;
        return Ok(());
    };

    let id = Uuid::new_v4().to_string();
    println!(
        "[TCP] commande depuis {client_addr} : recette « {name} », order_id={id}"
    );
    write_frame(stream, &order_receipt_frame(&id))?;
    println!("[TCP] order_id={id} accusé de réception (order_receipt) envoyé");

    let actions = flatten_actions(r);
    let mut ts = now_us();
    let mut updates = Vec::new();
    let mut content = String::new();
    let mut forwarded_to: Option<std::net::SocketAddr> = None;

    for (idx, a) in actions.iter().enumerate() {
        ts += 50;
        if state.has_capability(&a.name) {
            println!("[TCP] order_id={id} action locale : {}", a.name);
            let params: serde_json::Value =
                serde_json::to_value(&a.params).unwrap_or_else(|_| json!({}));
            updates.push(json!({
                "Action": {
                    "action": { "name": &a.name, "params": params },
                    "timestamp": ts
                }
            }));
            content.push_str(&content_line_for_action(&a.name));
        } else if let Some(target) = state.find_peer_with_capability(&a.name) {
            println!(
                "[TCP] order_id={id} capacité « {} » absente ici ; renvoi de la suite vers {target}",
                a.name
            );
            let tail: Vec<&str> = actions[idx..].iter().map(|x| x.name.as_str()).collect();
            println!(
                "[TCP] order_id={id} chaîne confiée à {target} (à partir de « {} », étapes : {tail:?})",
                a.name
            );
            updates.push(forward_to_json(target, ts));
            forwarded_to = Some(target);
            break;
        } else {
            println!(
                "[TCP] order_id={id} échec : aucun pair pour la capacité « {} »",
                a.name
            );
            write_frame(
                stream,
                &order_declined_frame(&format!("no peer for action: {}", a.name)),
            )?;
            return Ok(());
        }
    }

    if let Some(addr) = forwarded_to {
        println!(
            "[TCP] order_id={id} fin sur ce nœud ; la préparation continue sur {addr} (entrée Forward dans les mises à jour)"
        );
    } else {
        ts += 50;
        updates.push(json!({ "Deliver": { "timestamp": ts } }));
        println!("[TCP] order_id={id} livraison (Deliver) — toutes les étapes ont été faites sur ce nœud");
    }

    // `serde_json::json!` réserve `@` ; construire la clé `@@TAGGED@@` à la main.
    let mut order_id_tagged = serde_json::Map::new();
    order_id_tagged.insert(
        "@@TAGGED@@".to_string(),
        json!([37u64, id.as_str()]),
    );
    let inner = json!({
        "order_id": serde_json::Value::Object(order_id_tagged),
        "order_timestamp": now_us(),
        "content": content,
        "updates": updates,
    });
    let result_str = inner.to_string();
    write_frame(stream, &completed_order_frame(&name, &result_str))?;
    println!("[TCP] order_id={id} réponse finale (completed_order) envoyée à {client_addr}");
    Ok(())
}

fn handle_list_recipes(stream: &mut TcpStream, state: &GossipState, client_addr: &str) -> io::Result<()> {
    let n = state.recipes.len();
    println!("[TCP] list_recipes depuis {client_addr} ({n} recette(s))");
    write_frame(stream, &recipe_list_frame(state))?;
    Ok(())
}

fn handle_get_recipe(
    stream: &mut TcpStream,
    state: &GossipState,
    msg: &Value,
    client_addr: &str,
) -> io::Result<()> {
    let Some(name) = parse_recipe_name_get(msg) else {
        println!("[TCP] get_recipe invalide depuis {client_addr}");
        write_frame(stream, &order_declined_frame("invalid get_recipe"))?;
        return Ok(());
    };
    let Some(r) = state.recipes.get(&name) else {
        println!("[TCP] get_recipe « {name} » : inconnu (depuis {client_addr})");
        write_frame(
            stream,
            &order_declined_frame(&format!("unknown recipe: {name}")),
        )?;
        return Ok(());
    };
    println!("[TCP] get_recipe « {name} » depuis {client_addr} — envoi recipe_answer");
    write_frame(stream, &recipe_answer_frame(r))?;
    Ok(())
}

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
                    for (addr, info) in peers.iter() {
                        println!(
                            "  {addr}  counter={} generation={} caps={:?}",
                            info.version.counter, info.version.generation, info.capabilities
                        );
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
        println!("[TCP] Serveur en écoute sur {addr} (CBOR)");

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

fn handle_tcp_connection(mut stream: TcpStream, state: Arc<GossipState>) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
    loop {
        let msg = match read_frame(&mut stream) {
            Ok(m) => m,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                eprintln!("[TCP] lecture depuis {peer} : {e}");
                break;
            }
        };

        let outcome = match message_key(&msg) {
            Some("order") => handle_order(&mut stream, &state, &msg, &peer),
            Some("list_recipes") => handle_list_recipes(&mut stream, &state, &peer),
            Some("get_recipe") => handle_get_recipe(&mut stream, &state, &msg, &peer),
            Some(other) => {
                println!("[TCP] message ignoré (clé `{other}`) depuis {peer}");
                write_frame(
                    &mut stream,
                    &order_declined_frame(&format!("unsupported: {other}")),
                )
            }
            None => {
                write_frame(&mut stream, &order_declined_frame("invalid message"))
            }
        };

        if let Err(e) = outcome {
            eprintln!("[TCP] écriture vers {peer} : {e}");
            break;
        }
    }
}
