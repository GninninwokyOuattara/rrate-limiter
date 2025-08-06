use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::{Query, Request, State},
    response::IntoResponse,
    routing::get,
};

mod rate_limiter;
mod rules;
mod utils;

use axum_macros::debug_handler;
use matchit::Router as MatchitRouter;
use redis::Commands;

use crate::{rules::generate_dummy_rules, utils::populate_redis_kv_rule_algorithm};

struct States {
    redis_connection: redis::Connection,
    route_matcher: matchit::Router<String>,
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

    let states = Arc::new(Mutex::new(States {
        redis_connection: redis_connection,
        route_matcher,
    }));

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
    State(states): State<Arc<Mutex<States>>>,
    request: Request,
) -> impl IntoResponse {
    println!("Received params: {:?}", params);
    let uri = request.uri();
    let headers = request.headers();

    // Finding which pattern match the uri using the matcher

    let res = {
        let lock = states.lock().unwrap();
        lock.route_matcher.at(uri.path()).unwrap().value.clone()
    };

    let algorithm = {
        let mut lock = states.lock();
        /* println!("lock 2 found: {:#?}", &lock.err()); */
        let algorithm = lock
            .unwrap()
            .redis_connection
            .get::<&String, String>(&format!("rules_to_algorithms:{}", &res));
        println!("Algorithm found: {:#?}", algorithm);
    };

    /* let (message, headers) = rate_limiter::RateLimiter::check(
        &mut states.lock().unwrap().redis_connection,
        params.get("key").unwrap_or(&"default_key".to_string()),
        params.get("endpoint").unwrap_or(&"products".to_string()),
        rate_limiter::RateLimiterAlgorithms::FixedWindow,
    )
    .unwrap();

    (axum::http::StatusCode::OK, headers.to_headers(), message) */

    // println!("Request: {:#?}", request);

    "Hello World!"
}
