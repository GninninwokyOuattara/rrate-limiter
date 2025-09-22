use crate::server::run;
use rrl_core::tokio;

mod errors;
mod handler;
mod rate_limiter;
mod server;
mod server_state;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run().await
}
