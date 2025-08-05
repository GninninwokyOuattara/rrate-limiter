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
mod rules;
mod utils;

use matchit::Router as MatchitRouter;

struct States {
    redis_connection: redis::Connection,
    router: matchit::Router<&'static str>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut router = MatchitRouter::new();
    let st = String::from("/home");
    router.insert(st, "Welcome!")?;
    router.insert("/users/{id}".to_string(), "A User")?;

    tracing_subscriber::fmt::init();

    let states = Arc::new(Mutex::new(States {
        redis_connection: connect_to_redis()?,
        router,
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
