// TODO: Document on example of axum app from the git https://github.com/tokio-rs/axum/blob/main/examples/tokio-redis/src/main.rs
// TODO : Look well into the pool management bb8
// TODO : Implement with the connexion pool and benchmark again to see the difference. (hopefully some gains)

use anyhow::anyhow;
use axum::{
    Router,
    extract::{Request, State},
    response::IntoResponse,
    routing::get,
};
use axum_macros::debug_handler;
use matchit::Router as MatchitRouter;
use std::sync::Arc;
use tracing::info;

use bb8::{Pool, PooledConnection};
use bb8_redis::RedisConnectionManager;
use redis::{AsyncCommands, RedisError, aio::MultiplexedConnection};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod errors;
mod rate_limiter;
mod rules;
mod utils;

use crate::{
    errors::LimiterError,
    rate_limiter::{RateLimiter, RateLimiterAlgorithms, RateLimiterHeaders},
    rules::generate_dummy_rules,
    utils::{get_tracked_key_from_header, make_redis_key, populate_redis_with_rules},
};

struct States {
    route_matcher: Arc<matchit::Router<String>>,
    pool: MultiplexedConnection,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("connecting to redis");

    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let con = client.get_multiplexed_async_connection().await?;

    tracing::debug!("successfully connected to redis and pinged it");
    // let mut redis_connection = connect_to_redis()?;
    let mut route_matcher = MatchitRouter::new();

    let dummy_rules = generate_dummy_rules();

    // populate_redis_kv_rule_algorithm(&mut redis_connection, &dummy_rules)?;
    populate_redis_with_rules(con.clone(), &dummy_rules)
        .await
        .unwrap();

    // Here we are just building the route matcher.
    dummy_rules.into_iter().for_each(|rule| {
        route_matcher
            .insert(rule.route, rule.hash)
            .expect("Failed to insert route");
    });

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let states = Arc::new(States {
        route_matcher: Arc::new(route_matcher),
        pool: con,
    });

    let app = Router::new()
        .route("/", get(limiter_handler))
        .fallback(get(limiter_handler))
        .with_state(states.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

// Redis connection here.

fn connect_to_redis() -> Result<redis::Connection, Box<dyn std::error::Error>> {
    Ok(redis::Client::open("redis://127.0.0.1/")?.get_connection()?)
}

#[debug_handler]
async fn limiter_handler(
    State(states): State<Arc<States>>,
    request: Request,
) -> anyhow::Result<impl IntoResponse, LimiterError> {
    let uri = request.uri();
    let _headers = request.headers();

    // Finding which pattern match the uri using the matcher

    // We use the router matcher for that
    let matched_route = states
        .route_matcher
        .clone()
        .at(uri.path())
        .map_err(|_err| LimiterError::NoRouteMatch(uri.path().to_string()))?
        .value
        .clone();

    /* */
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

    let tracking_key = get_tracked_key_from_header(
        &request.headers(),
        &tracking_type.into(),
        custom_tracking_key.into(),
    )?;

    // Where the rate limiting happens.
    let Ok(rate_limiting_algorithm) = RateLimiterAlgorithms::from_string(&rl_algo) else {
        return Err(anyhow!("Could not convert cache key to local algorithm").into());
    };

    let (message, headers) = check_strong(
        states.pool.clone(),
        &tracking_key,
        &matched_route,
        rate_limiting_algorithm,
        limit,
        expiration,
    )
    .await?;

    match message.as_str() {
        "Rate limit exceeded." => {
            return Ok((
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                headers.to_headers(),
                message,
            ));
        }
        "Rate limit not exceeded." => {
            return Ok((axum::http::StatusCode::OK, headers.to_headers(), message));
        }
        _ => {
            return Ok((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                headers.to_headers(),
                message,
            ));
        }
    }

    // Ok((axum::http::StatusCode::OK, headers.to_headers(), message))
}

pub async fn check_strong(
    mut pool: MultiplexedConnection,
    tracked_key: &str,
    hashed_route: &str,
    algorithm: RateLimiterAlgorithms,
    limit: u64,
    expiration: u64,
) -> Result<(String, RateLimiterHeaders), RedisError> {
    let redis_key = make_redis_key(tracked_key, hashed_route, &algorithm);
    let script = redis::Script::new(algorithm.get_script());

    let result: Vec<String> = script
        .key(redis_key)
        .arg(limit)
        .arg(expiration)
        .invoke_async(&mut pool)
        .await
        .unwrap();

    Ok((
        result[3].clone(),
        RateLimiterHeaders::new(
            result[0].parse().unwrap_or_default(),
            result[1].parse().unwrap_or_default(),
            result[2].parse().unwrap_or_default(),
            algorithm.to_string(),
        ),
    ))
}
