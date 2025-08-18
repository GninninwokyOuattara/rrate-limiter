use axum::{
    Router,
    extract::{Request, State},
    response::IntoResponse,
    routing::get,
};
use axum_macros::debug_handler;
use matchit::Router as MatchitRouter;
use parking_lot::RwLock;
use rrl_core::{
    LimiterTrackingType, RateLimiterAlgorithms,
    tokio_postgres::{Client, NoTls},
    tracing, tracing_subscriber,
};
use std::{mem, sync::Arc};

use anyhow::anyhow;
use redis::{AsyncCommands, aio::ConnectionManager};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    errors::LimiterError,
    rate_limiter::execute_rate_limiting,
    utils::{
        generate_dummy_rules, get_rules_from_redis, get_tracked_key_from_header,
        instantiate_matcher_with_rules, populate_redis_with_rules,
    },
};

mod errors;
mod rate_limiter;
mod utils;

struct States {
    route_matcher: Arc<RwLock<matchit::Router<String>>>,
    pool: ConnectionManager,
    pg_client: Client,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = std::env::var("RL_POSTGRES_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("RL_POSTGRES_PORT").unwrap_or("5432".to_string());
    let user = std::env::var("RL_POSTGRES_USER").unwrap_or("postgres".to_string());
    let password = std::env::var("RL_POSTGRES_PASSWORD").unwrap_or("postgres".to_string());
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::debug!("connecting to postgres...");
    let (pg_client, connection) = rrl_core::tokio_postgres::connect(
        format!("host={host} port={port} user={user} password={password} dbname=rrate-limiter")
            .as_str(),
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("connection error: {}", e);
        }
    });

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
        pg_client,
    });

    tracing::debug!("Starting server on port 3000");

    let app = Router::new()
        .route("/", get(limiter_handler))
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
    let matched_route = states
        .route_matcher
        .clone()
        .read()
        .at(request.uri().path())
        .map_err(|_err| LimiterError::NoRouteMatch(request.uri().path().to_string()))?
        .value
        .clone();

    // We retrieve the algorithm, expiration and limit from redis
    let (rl_algo, expiration, limit, custom_tracking_key, tracking_type): (
        String,
        u64,
        u64,
        String,
        String,
    ) = states
        .pool
        .clone()
        .hmget(
            format!("rules:{}", matched_route),
            &[
                "algorithm",
                "expiration",
                "limit",
                "custom_tracking_key",
                "tracking_type",
            ],
        )
        .await?;

    let tracking_type: LimiterTrackingType =
        tracking_type.try_into().map_err(|err| anyhow!("{err}"))?;

    let tracking_key = get_tracked_key_from_header(
        request.headers(),
        &tracking_type,
        custom_tracking_key.into(),
    )?;

    // Where the rate limiting happens.
    let Ok(rate_limiting_algorithm) = RateLimiterAlgorithms::from_string(&rl_algo) else {
        return Err(anyhow!("Could not convert cache key to local algorithm").into());
    };

    let (message, headers) = execute_rate_limiting(
        states.pool.clone(),
        &tracking_key,
        &matched_route,
        rate_limiting_algorithm,
        limit,
        expiration,
    )
    .await?;

    match message.as_str() {
        "Rate limit exceeded." => Ok((
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            headers.to_headers(),
            message,
        )),
        "Rate limit not exceeded." => {
            Ok((axum::http::StatusCode::OK, headers.to_headers(), message))
        }
        _ => Ok((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            headers.to_headers(),
            message,
        )),
    }
}

// TODO: It is wrong to return 404 when a url is not found in the matcher. a 200 should be returned. 404 is meant to indicate that the resources was not found.

// TODO: use ttl from the rule field to set a ttl for the rule allowing refresh when user update the configuration

// TODO: implement read from db in case of cache misses and write back to redis db
