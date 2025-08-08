use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use axum::{
    Router,
    extract::{Query, Request, State},
    response::IntoResponse,
    routing::get,
};

mod errors;
mod rate_limiter;
mod rules;
mod utils;

use axum_macros::debug_handler;
use matchit::Router as MatchitRouter;
use redis::Commands;

use crate::{
    rate_limiter::RateLimiterAlgorithms,
    rules::generate_dummy_rules,
    utils::{get_tracked_key_from_header, populate_redis_with_rules},
};

struct States {
    redis_connection: Arc<Mutex<redis::Connection>>,
    route_matcher: Arc<Mutex<matchit::Router<String>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut redis_connection = connect_to_redis()?;
    let mut route_matcher = MatchitRouter::new();

    let dummy_rules = generate_dummy_rules();

    // populate_redis_kv_rule_algorithm(&mut redis_connection, &dummy_rules)?;
    populate_redis_with_rules(&mut redis_connection, &dummy_rules)?;

    // Here we are just building the route matcher.
    dummy_rules.into_iter().for_each(|rule| {
        route_matcher
            .insert(rule.route, rule.hash)
            .expect("Failed to insert route");
    });

    tracing_subscriber::fmt::init();

    let states = Arc::new(States {
        redis_connection: Arc::new(Mutex::new(redis_connection)),
        route_matcher: Arc::new(Mutex::new(route_matcher)),
    });

    let app = Router::new()
        .route("/", get(limiter_handler))
        .fallback(get(limiter_handler))
        .with_state(states.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// Redis connection here.

fn connect_to_redis() -> Result<redis::Connection, Box<dyn std::error::Error>> {
    Ok(redis::Client::open("redis://127.0.0.1/")?.get_connection()?)
}

#[debug_handler]
async fn limiter_handler(
    Query(params): Query<HashMap<String, String>>,
    State(states): State<Arc<States>>,
    request: Request,
) -> anyhow::Result<impl IntoResponse, errors::LimiterError> {
    println!("Received params: {:#?}", params);
    let uri = request.uri();
    let _headers = request.headers();
    println!("Headers :: {:?}", _headers);
    // let tracking_key = get_tracked_key_from_header(headers, tracking_type, custom_header_key)

    // TODO: properly handle the key that should be rate limited.

    // Finding which pattern match the uri using the matcher

    // We use the router matcher for that
    let matched_route = states
        .route_matcher
        .lock()?
        .at(uri.path())
        .map_err(|_err| errors::LimiterError::NoRouteMatch(uri.path().to_string()))?
        .value
        .clone();
    println!("The matching route : {:#?}", &matched_route);

    // We retrieve the algorithm, expiration and limit from redis
    let (rl_algo, expiration, limit, custom_tracking_key, tracking_type): (
        String,
        u64,
        u64,
        String,
        String,
    ) = states.redis_connection.lock()?.hmget(
        format!("rules:{}", matched_route),
        &[
            "algorithm",
            "expiration",
            "limit",
            "custom_tracking_key",
            "tracking_type",
        ],
    )?;

    println!(
        "All values : {:?}",
        (
            &rl_algo,
            &expiration,
            &limit,
            &custom_tracking_key,
            &tracking_type
        )
    );

    let tracking_key = get_tracked_key_from_header(
        &request.headers(),
        &tracking_type.into(),
        custom_tracking_key.into(),
    )?;

    println!("The tracking key extracted is :: {}", &tracking_key);

    // Where the rate limiting happens.
    let Ok(rate_limiting_algorithm) = RateLimiterAlgorithms::from_string(&rl_algo) else {
        return Err(anyhow!("Could not convert cache key to local algorithm").into());
    };

    let (message, headers) = rate_limiter::RateLimiter::check(
        &mut states.redis_connection.lock().unwrap(),
        &tracking_key,
        &matched_route,
        rate_limiting_algorithm,
        limit,
        expiration,
    )?;

    Ok((axum::http::StatusCode::OK, headers.to_headers(), message))
}
