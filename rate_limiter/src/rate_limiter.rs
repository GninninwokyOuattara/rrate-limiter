use redis::{RedisError, aio::ConnectionManager};
use rrl_core::RateLimiterAlgorithms;

use crate::utils::make_redis_key;

const FIXED_WINDOW: &str = "fw";
const SLIDING_WINDOW_COUNTER: &str = "swc";
const SLIDING_WINDOW_LOG: &str = "swl";
const LEAKY_BUCKET: &str = "lb";
const TOKEN_BUCKET: &str = "tb";

pub struct RateLimiterHeaders {
    limit: u64,     // Maximum number of requests allowed
    remaining: u64, // Number of requests remaining in the current window
    reset: u64,     // Time in seconds until the rate limit resets
    policy: String, // The rate limiting policy used
}

impl RateLimiterHeaders {
    pub fn new(limit: u64, remaining: u64, reset: u64, policy: String) -> Self {
        Self {
            limit,
            remaining,
            reset,
            policy,
        }
    }

    pub fn to_headers(&self) -> axum::http::HeaderMap {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-RateLimit-Limit", self.limit.to_string().parse().unwrap());
        headers.insert(
            "X-RateLimit-Remaining",
            self.remaining.to_string().parse().unwrap(),
        );
        headers.insert("X-RateLimit-Reset", self.reset.to_string().parse().unwrap());
        headers.insert("X-RateLimit-Policy", self.policy.parse().unwrap());
        // headers.insert("Retry-After", self.retry_after.to_string().parse().unwrap());
        headers
    }
}

pub async fn execute_rate_limiting(
    mut pool: ConnectionManager,
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
