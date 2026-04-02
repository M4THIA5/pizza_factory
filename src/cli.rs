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
        addr: String,

        #[arg(short, long)]
        peer: Vec<String>,

        #[arg(short, long)]
        capabilities: Vec<String>,

        #[arg(short, long, default_value = "assets/pizza.recipes")]
        recipes: String,

        #[arg(short, long, default_value_t = 10)]
        gossip_interval: u64,
    },
    Client {
        #[arg(short, long)]
        pizza: String,
    },
}