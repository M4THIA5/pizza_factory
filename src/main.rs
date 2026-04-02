mod cli;
mod protocol;
mod recipe;
mod server;

use cli::{Args, Command};
use clap::Parser;

use server::run_server;

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Server { addr, peer, capabilities, recipes, gossip_interval } => {
            run_server(addr, peer, capabilities, recipes, gossip_interval)
        }
        Command::Client { pizza: _ } => { todo!() }
    }
}
