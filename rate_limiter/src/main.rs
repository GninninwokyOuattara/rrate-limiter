use axum::{
    Router,
    extract::{Request, State},
    response::IntoResponse,
    routing::get,
};
use axum_macros::debug_handler;
use matchit::Router as MatchitRouter;
use rrl_core::{LimiterTrackingType, RateLimiterAlgorithms, tracing, tracing_subscriber};
use std::sync::Arc;

use anyhow::anyhow;
use redis::{AsyncCommands, aio::ConnectionManager};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    errors::LimiterError,
    rate_limiter::execute_rate_limiting,
    utils::{generate_dummy_rules, get_tracked_key_from_header, populate_redis_with_rules},
};

mod errors;
mod rate_limiter;
mod utils;

struct States {
    route_matcher: Arc<matchit::Router<String>>,
    pool: ConnectionManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::debug!("connecting to redis...");
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let redis_connection = ConnectionManager::new(client).await?;
    tracing::debug!("Managed connection to redis established.");

    tracing::debug!("generating dummy rules...");
    let dummy_rules = generate_dummy_rules();

    // populate_redis_kv_rule_algorithm(&mut redis_connection, &dummy_rules)?;
    tracing::debug!("populating redis with generatedrules...");
    populate_redis_with_rules(redis_connection.clone(), &dummy_rules)
        .await
        .unwrap();

    // Here we are just building the route matcher.
    tracing::debug!("building sample route matcher...");
    let mut route_matcher = MatchitRouter::new();
    dummy_rules.into_iter().for_each(|rule| {
        route_matcher
            .insert(rule.route, rule.id)
            .expect("Failed to insert route");
    });

    let states = Arc::new(States {
        route_matcher: Arc::new(route_matcher),
        pool: redis_connection,
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
        .at(&request.uri().path())
        .map_err(|_err| LimiterError::NoRouteMatch(request.uri().path().to_string()))?
        .value
        .clone();

    // TODO : A preamptive check to see if the rate limit is already reached. Will allow for early return.

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
        &request.headers(),
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
}

// TODO: It is wrong to return 404 when a url is not found in the matcher. a 200 should be returned. 404 is meant to indicate that the resources was not found.
