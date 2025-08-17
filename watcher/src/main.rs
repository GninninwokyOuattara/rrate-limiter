use std::sync::Arc;

use redis::aio::ConnectionManager;
use rrl_core::{
    chrono,
    db::{get_all_rules_updated_at_and_after_date, get_last_update_time},
    tokio_postgres::NoTls,
    tracing, tracing_subscriber,
};
use tokio::time::{self, Duration, sleep};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::debug!("Reading environment variables...");

    let host = std::env::var("RL_POSTGRES_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("RL_POSTGRES_PORT").unwrap_or("5432".to_string());
    let user = std::env::var("RL_POSTGRES_USER").unwrap_or("postgres".to_string());
    let password = std::env::var("RL_POSTGRES_PASSWORD").unwrap_or("postgres".to_string());
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());

    tracing::debug!("Connecting to postgres...");

    let (pg_client, connection) = rrl_core::tokio_postgres::connect(
        format!("host={host} port={port} user={user} password={password} dbname=rrate-limiter")
            .as_str(),
        NoTls,
    )
    .await?;
    let pg_client = Arc::new(pg_client);
    tracing::debug!("Connected to postgres. Spawning pg connection manager task...");
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    tracing::debug!("connecting to redis...");
    let client = redis::Client::open(format!("redis://{}:{}", redis_host, redis_port))?;
    let redis_client = ConnectionManager::new(client).await?;
    tracing::debug!("Managed connection to redis established.");

    let mut interval = time::interval(Duration::from_secs(60));
    let mut cursor = get_last_update_time(pg_client.clone())
        .await
        .unwrap()
        .unwrap_or_default();

    loop {
        tracing::debug!("Checking for new at or after updates {:?}...", cursor);
        let rules = get_all_rules_updated_at_and_after_date(pg_client.clone(), cursor).await;
        if let Ok(rules) = rules
            && rules.len() > 0
        {
            tracing::debug!("Found {} rules to update", rules.len());
            tracing::debug!("Rules :: {:#?}", rules);

            cursor = rules.last().unwrap().date_modification + chrono::Duration::microseconds(1);
            tracing::debug!("Updating cursor to: {}", cursor);
            // populate_redis_with_rules(redis_client.clone(), rules).await;
        }
        tracing::debug!("Sleeping for 60 seconds...");
        interval.tick().await;
    }

    Ok(())
}
