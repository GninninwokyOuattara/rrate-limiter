use rrl_core::tracing_subscriber;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = std::env::var("RL_POSTGRES_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("RL_POSTGRES_PORT").unwrap_or("5432".to_string());
    let user = std::env::var("RL_POSTGRES_USER").unwrap_or("postgres".to_string());
    let password = std::env::var("RL_POSTGRES_PASSWORD").unwrap_or("postgres".to_string());
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());
}
