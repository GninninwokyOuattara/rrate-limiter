use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, patch, post},
};
use rrl_core::{
    tokio_postgres::{self, NoTls},
    tracing,
    tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt},
};

use crate::handlers::{delete_rule, get_rule_by_id, get_rules, patch_rule, post_rule};

mod errors;
mod handlers;
mod models;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (client, connection) = tokio_postgres::connect(
        "host=localhost user=postgres password=postgres dbname=rrate-limiter",
        NoTls,
    )
    .await?;

    let client = Arc::new(client);

    // The connection object performs the actual communication with the database,
    // so spawn it off to run on its own.
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

    tracing::debug!("Connected to Postgres");

    let app = Router::new()
        .route("/", get(async move || "Hello, World!"))
        .route("/rules", get(get_rules))
        .route("/rules/{rule_id}", get(get_rule_by_id))
        .route("/rules", post(post_rule))
        .route("/rules/{rule_id}", patch(patch_rule))
        .route("/rules/{rule_id}", delete(delete_rule))
        .with_state(client.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
