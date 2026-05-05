use clap::{Parser, CommandFactory};
use wind_merge::client::load_client;
use wind_merge::server::server;
use wind_merge::worker::worker;
use wind_merge::cli::{Cli, Command};

#[rocket::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.mode()? {
        None | Some(Command::Server) => server::run_server().await?,
        Some(Command::Worker) => worker::run_worker()?,
        Some(Command::Client(args)) => load_client::run(args.into()).map(|_| ())?,
        Some(Command::Help) => {
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}