use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{Context, anyhow};
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
    rate_limiter::RateLimiterAlgorithms, rules::generate_dummy_rules,
    utils::populate_redis_kv_rule_algorithm,
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

    populate_redis_kv_rule_algorithm(&mut redis_connection, &dummy_rules)?;

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
    println!("Received params: {:?}", params);
    let uri = request.uri();
    let headers = request.headers();

    // Finding which pattern match the uri using the matcher

    // We use the router matcher for that
    let matched_route = states.route_matcher.lock()?.at(uri.path())?.value.clone();
    println!("The matching route : {:#?}", &matched_route);

    // Find the algorithm for that route in the redis cache.

    let algorithm = states
        .redis_connection
        .lock()?
        .get::<&String, String>(&format!("rules_to_algorithms:{}", &matched_route))
        .context("Failed to succesfully retrieve the redis key")?;

    println!("Algorithm found: {:#?}", algorithm);

    // Where the rate limiting happens.
    let Ok(algo) = RateLimiterAlgorithms::from_string(&algorithm) else {
        return Err(anyhow!("Could not convert cache key to local algorithm").into());
    };

    let (message, headers) = rate_limiter::RateLimiter::check(
        &mut states.redis_connection.lock().unwrap(),
        &"limitkey",
        &matched_route,
        algo,
        100,
        3600,
    )
    .unwrap();

    Ok((axum::http::StatusCode::OK, headers.to_headers(), message))

    // println!("Request: {:#?}", request);

    // "Hello World!"
}
