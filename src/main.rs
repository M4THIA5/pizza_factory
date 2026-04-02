mod cli;
mod protocol;
mod server;

use cli::{Args, Command};
use clap::Parser;

use server::run_server;

fn main() {
    let args = Args::parse();
    match args.command {
        Command::Server { addr, peer } => { run_server(addr, peer) }
        Command::Client { pizza: _ } => { todo!() }
    }
}
