use axum::{
    Router,
    extract::{Request, State},
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
};
use axum_macros::debug_handler;
use parking_lot::RwLock;
use rrl_core::{tracing, tracing_subscriber};
use std::sync::Arc;

use redis::aio::ConnectionManager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    errors::LimiterError,
    rate_limiter::execute_rate_limiting,
    utils::{
        get_rules_from_redis, get_rules_information_by_redis_json_key, get_tracked_key_from_header,
        instantiate_matcher_with_rules,
    },
};

mod errors;
mod rate_limiter;
mod utils;

struct States {
    route_matcher: Arc<RwLock<matchit::Router<String>>>,
    pool: ConnectionManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::debug!("connecting to redis...");
    let client = redis::Client::open(format!(
        "redis://{}:{}?protocol=resp3",
        redis_host, redis_port
    ))?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let config = redis::aio::ConnectionManagerConfig::new()
        .set_push_sender(tx)
        .set_automatic_resubscription();
    let mut redis_connection = client.get_connection_manager_with_config(config).await?;
    let mut con_for_task = redis_connection.clone();
    tracing::debug!("Managed connection to redis established.");
    redis_connection.subscribe("rl_update").await.unwrap(); // We actually want to fails if it is impossible to subscribe initially.
    tracing::debug!("Subscribed to rl_update channel.");

    let rules_config = get_rules_from_redis(&mut redis_connection).await.unwrap();
    let route_matcher = Arc::new(RwLock::new(instantiate_matcher_with_rules(rules_config))); // Initial instance of the matcher.
    let route_matcher_for_task = Arc::clone(&route_matcher);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracing::debug!("Received message from redis: {msg:?}");
            let new_rules = get_rules_from_redis(&mut con_for_task).await.unwrap();
            let new_router = instantiate_matcher_with_rules(new_rules);
            *route_matcher_for_task.write() = new_router;
            tracing::debug!("Updated route matcher.");
        }
    });

    let states = Arc::new(States {
        route_matcher: route_matcher.clone(),
        pool: redis_connection,
    });

    tracing::debug!("Starting server on port 3000");

    let app = Router::new()
        .route("/", get(async move || "WAAAGH!"))
        .fallback(get(limiter_handler))
        .with_state(states.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

#[debug_handler]
async fn limiter_handler(
    State(states): State<Arc<States>>,
    request: Request,
) -> anyhow::Result<impl IntoResponse, LimiterError> {
    // Finding which pattern match the uri using the matcher
    let associated_key = states
        .route_matcher
        .clone()
        .read()
        .at(request.uri().path())
        .map_err(|_err| LimiterError::NoRouteMatch(request.uri().path().to_string()))?
        .value
        .clone();

    // We retrieve the algorithm, expiration and limit from redis
    let limiter_rule =
        get_rules_information_by_redis_json_key(&mut states.pool.clone(), &associated_key)
            .await
            .unwrap();

    // In case the rule is disabled
    if let Some(v) = &limiter_rule.active
        && *v == false
    {
        return Ok((
            axum::http::StatusCode::OK,
            HeaderMap::default(),
            "Rate limit not exceeded.".to_string(),
        ));
    }

    let tracking_key = get_tracked_key_from_header(
        request.headers(),
        &limiter_rule.tracking_type,
        limiter_rule.custom_tracking_key,
    )?;

    let headers = execute_rate_limiting(
        states.pool.clone(),
        &tracking_key,
        &associated_key,
        limiter_rule.algorithm,
        limiter_rule.limit as u64,
        limiter_rule.expiration as u64,
    )
    .await?;

    Ok((
        axum::http::StatusCode::OK,
        headers.to_headers(),
        "Rate limit not exceeded.".to_string(),
    ))
}

// TODO: fix sorted set not having a ttl resulting in the data staying in redis forever
