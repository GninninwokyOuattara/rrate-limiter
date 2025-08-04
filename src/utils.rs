use crate::rate_limiter::RateLimiterAlgorithms;

pub fn make_redis_key(key: &str, endpoint: &str, algorithm: &RateLimiterAlgorithms) -> String {
    format!("{}:{}:{}", algorithm.to_string(), key, endpoint)
}
