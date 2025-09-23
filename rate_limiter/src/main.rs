use std::path::PathBuf;

use crate::{configurations_loader::load_configuration, server::run};
use clap::{Parser, Subcommand};
use tokio;

mod configurations_loader;
mod errors;
mod handler;
mod rate_limiter;
mod rules;
mod server;
mod server_state;
mod utils;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "A simple and efficient rate limiter that supports five well-known algorithms: Fixed Window, Sliding Window Log, Sliding Window Counter, Leaky Bucket, and Token Bucket. 
It easy to setup, configure and is design to be easily scallable."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run as a rate limiter instance.
    Run,
    /// Load configuration file into the redis instance used by the rate limiters.
    Load {
        /// lists test values
        #[arg(short, long)]
        file: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Run => run().await?,
        Commands::Load { file } => load_configuration(file).await?,
    }

    // run().await
    Ok(())
}
