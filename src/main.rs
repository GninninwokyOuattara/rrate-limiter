use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};

mod rate_limiter;
mod utils;

struct States {
    redis_connection: redis::Connection,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let states = Arc::new(Mutex::new(States {
        redis_connection: connect_to_redis()?,
    }));

    let app = Router::new()
        .route("/", get(|| async { "hello, world!" }))
        .route("/check", get(limiter_handler))
        .with_state(states.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// Redis connection here.

fn connect_to_redis() -> Result<redis::Connection, Box<dyn std::error::Error>> {
    Ok(redis::Client::open("redis://127.0.0.1/")?.get_connection()?)
}

async fn limiter_handler(
    Query(params): Query<HashMap<String, String>>,
    State(states): State<Arc<Mutex<States>>>,
) -> impl IntoResponse {
    println!("Received params: {:?}", params);
    let (message, headers) = rate_limiter::RateLimiter::check(
        &mut states.lock().unwrap().redis_connection,
        params.get("key").unwrap_or(&"default_key".to_string()),
        params.get("endpoint").unwrap_or(&"products".to_string()),
        rate_limiter::RateLimiterAlgorithms::FixedWindow,
    )
    .unwrap();

    (axum::http::StatusCode::OK, headers.to_headers(), message)
}
