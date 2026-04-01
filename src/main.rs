mod cli;

use std::time::{SystemTime, UNIX_EPOCH};

use cli::{Args, Command};
use clap::Parser;

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Server { addr, peer } => { run_server(addr, peer) }
        Command::Client { pizza: _ } => { todo!() }
    }
}

fn run_server(addr: String, initial_peers: Vec<String>) {
    let generation = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    println!("Agent démarré : {addr} (generation={generation})");

    
}
