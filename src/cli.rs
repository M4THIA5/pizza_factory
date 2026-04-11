use std::net::SocketAddr;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Server {
        #[arg(short, long, default_value = "127.0.0.1:8000")]
        addr: SocketAddr,

        #[arg(short, long)]
        peer: Vec<SocketAddr>,

        #[arg(short, long)]
        capabilities: Vec<String>,

        #[arg(short, long, default_value = "assets/pizza.recipes")]
        recipes: String,

        #[arg(short, long, default_value_t = 10)]
        gossip_interval: u64,
    },
    Client {
        #[arg(long)]
        peer: SocketAddr,
        #[command(subcommand)]
        client: ClientCommand,
    },
}

#[derive(Subcommand)]
pub enum ClientCommand {
    /// Commande une recette sur le pair (équivalent à `pizza_factory client … order …`)
    Order {
        /// Nom de la recette (ex. `Minimal`)
        recipe: String,
    },
    #[command(name = "list-recipes")]
    ListRecipes,
    #[command(name = "get-recipe")]
    GetRecipe {
        recipe: String,
    },
}
