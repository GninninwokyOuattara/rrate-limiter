use hyper_util::rt::TokioIo;
use parking_lot::RwLock;
use rrl_core::{
    redis::{self},
    tokio::{self, net::TcpListener},
    tracing,
    tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt},
};
use std::sync::Arc;

use crate::{
    handler::limiter_handler,
    server_state::States,
    utils::{get_rules_from_redis, instantiate_matcher_with_rules},
};

use std::net::SocketAddr;

use hyper::{server::conn::http1, service::service_fn};

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());
    // TODO: password for redis

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("connecting to redis...");
    let client = redis::Client::open(format!(
        "redis://{}:{}?protocol=resp3",
        redis_host, redis_port
    ))?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let config = redis::aio::ConnectionManagerConfig::new()
        .set_push_sender(tx)
        .set_connection_timeout(std::time::Duration::from_secs(2))
        .set_number_of_retries(1)
        .set_automatic_resubscription();

    let mut redis_connection = client.get_connection_manager_with_config(config).await?;
    let mut con_for_task = redis_connection.clone();
    tracing::info!("Managed connection to redis established.");
    redis_connection.subscribe("rl_update").await.unwrap(); // We actually want to fails if it is impossible to subscribe initially.
    tracing::info!(
        "Subscribed to rl_update channel. Updates will trigger a rebuild of the matcher."
    );

    let rules_config = get_rules_from_redis(&mut redis_connection).await.unwrap();
    let route_matcher = Arc::new(RwLock::new(instantiate_matcher_with_rules(rules_config))); // Initial instance of the matcher.
    let route_matcher_for_task = Arc::clone(&route_matcher);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracing::info!("Event received: {msg:?}");
            let new_rules = get_rules_from_redis(&mut con_for_task).await.unwrap();
            let length = new_rules.len();
            let new_router = instantiate_matcher_with_rules(new_rules);
            *route_matcher_for_task.write() = new_router;
            tracing::info!("Matcher has been rebuilt with {length} routes.");
        }
    });

    let states = Arc::new(States {
        route_matcher: route_matcher.clone(),
        pool: redis_connection,
    });

    tracing::info!("Starting server on port 3000");

    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let states = states.clone();

        tokio::spawn(async move {
            let _ = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let states = states.clone();
                        limiter_handler(states, req)
                    }),
                )
                .await;
        });
    }
}
