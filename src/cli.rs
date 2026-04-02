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
    },
    Client {
        #[arg(short, long)]
        pizza: String,
    },
}