mod cli;
mod client;
mod protocol;
mod recipe;
mod server;
mod wire;

use cli::{Args, ClientCommand, Command};
use clap::Parser;

use server::run_server;

fn main() {
    let args = Args::parse();
    if let Err(e) = run(&args) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(args: &Args) -> Result<(), String> {
    match &args.command {
        Command::Server {
            addr,
            peer,
            capabilities,
            recipes,
            gossip_interval,
        } => {
            run_server(*addr, peer.clone(), capabilities.clone(), recipes.clone(), *gossip_interval);
            Ok(())
        }
        Command::Client { peer, client } => match client {
            ClientCommand::Order { recipe } => client::run_order(*peer, recipe),
            ClientCommand::ListRecipes => client::run_list_recipes(*peer),
            ClientCommand::GetRecipe { recipe } => client::run_get_recipe(*peer, recipe),
        },
    }
}
