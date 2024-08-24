use crate::{commands::Commands};
use clap::Parser;

mod commands;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let args = commands::Args::parse();
    match args.command {
        Commands::Server(server) => server.run().await?,
    }

    Ok(())
}
