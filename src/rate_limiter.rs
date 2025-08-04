use std::result;

use crate::utils::make_redis_key;

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

pub enum RateLimiterAlgorithms {
    FixedWindow,
    SlidingWindowCounter,
    SlidingWindowLog,
    TokenBucket,
    LeakyBucket,
}

impl RateLimiterAlgorithms {
    pub fn to_string(&self) -> String {
        match self {
            RateLimiterAlgorithms::FixedWindow => "fixed_window".to_string(),
            RateLimiterAlgorithms::SlidingWindowCounter => "sliding_window_counter".to_string(),
            RateLimiterAlgorithms::SlidingWindowLog => "sliding_window_log".to_string(),
            RateLimiterAlgorithms::TokenBucket => "token_bucket".to_string(),
            RateLimiterAlgorithms::LeakyBucket => "leaky_bucket".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        match s {
            "fixed_window" => Some(RateLimiterAlgorithms::FixedWindow),
            "sliding_window_counter" => Some(RateLimiterAlgorithms::SlidingWindowCounter),
            "sliding_window_log" => Some(RateLimiterAlgorithms::SlidingWindowLog),
            "token_bucket" => Some(RateLimiterAlgorithms::TokenBucket),
            "leaky_bucket" => Some(RateLimiterAlgorithms::LeakyBucket),
            _ => None,
        }
    }

    pub fn get_script(&self) -> &'static str {
        match self {
            RateLimiterAlgorithms::FixedWindow => {
                r#"
                local key = KEYS[1]
                local limit = tonumber(ARGV[1])

    
                if redis.call('EXISTS', key) == 0 then
                    redis.call('SET', key, 0)
                    redis.call('EXPIRE', key, ARGV[1])
                end

                if redis.call('GET', key) + 1 > limit then
                    local remaining = limit - redis.call('GET', key)
                    local reset = redis.call('TTL', key)
                    return {
                        limit,
                        remaining,
                        reset,
                        'Rate limit exceeded.',
                    }

                else
                    redis.call('INCR', key)
                    local remaining = limit - redis.call('GET', key)
                    local reset = redis.call('TTL', key)
                    return {
                        limit,
                        remaining,
                        reset,
                        'Rate limit not exceeded.',
                    }
                end

                "#
            }
            // Other algorithms can be implemented similarly
            _ => "",
        }
    }
}

pub struct RateLimiter {}

impl RateLimiter {
    pub fn check(
        mut redis_connection: &mut redis::Connection,
        key: &str,
        endpoint: &str,
        algorithm: RateLimiterAlgorithms,
    ) -> Result<(String, RateLimiterHeaders), ()> {
        let redis_key = make_redis_key(key, endpoint, &algorithm);

        let script = redis::Script::new(algorithm.get_script());

        let redis_result = script
            .key(redis_key)
            .arg(100)
            .invoke::<Vec<String>>(&mut redis_connection);

        let result = if let Ok(result) = redis_result {
            result
        } else {
            return Err(());
        };

        Ok((
            result[3].clone(),
            RateLimiterHeaders::new(
                result[0].parse().unwrap_or(0),
                result[1].parse().unwrap_or(0),
                result[2].parse().unwrap_or(0),
                algorithm.to_string(),
            ),
        ))
    }
}
