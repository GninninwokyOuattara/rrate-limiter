use redis::aio::ConnectionManager;
use rrl_core::{RateLimiterAlgorithms, tracing};

use crate::{errors::LimiterError, utils::make_redis_key};

#[derive(Debug)]
pub struct RateLimiterHeaders {
    pub limit: u64,     // Maximum number of requests allowed
    pub remaining: u64, // Number of requests remaining in the current window
    pub reset: u64,     // Time in seconds until the rate limit resets
    pub policy: String, // The rate limiting policy used
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
}

pub async fn execute_rate_limiting(
    mut pool: ConnectionManager,
    tracked_key: &str,
    rule_redis_config_key: &str,
    algorithm: RateLimiterAlgorithms,
    limit: u64,
    expiration: u64,
    route: String,
) -> Result<RateLimiterHeaders, LimiterError> {
    tracing::debug!(
        "Executing rate limiting with key {tracked_key}, algorithm {algorithm:?}, limit {limit}, expiration {expiration} and rule_redis_config_key {rule_redis_config_key}"
    );
    let redis_key = make_redis_key(tracked_key, rule_redis_config_key, &algorithm);
    let script = redis::Script::new(algorithm.get_script());

    let result: Vec<u64> = script
        .key(redis_key)
        .arg(limit)
        .arg(expiration)
        .invoke_async(&mut pool)
        .await?;

    let headers = RateLimiterHeaders::new(result[0], result[1], result[2], algorithm.to_string());
    tracing::debug!("Resulting headers after rate limiting: {:#?}", headers);

    if result[3] == 0 {
        return Err(LimiterError::RateLimitExceeded {
            headers,
            key: tracked_key.to_string(),
            msg: "Rate limit exceeded".to_string(),
            route,
        });
    }

    Ok(headers)
}
