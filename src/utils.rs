use redis::{Commands, RedisError};

use crate::{rate_limiter::RateLimiterAlgorithms, rules::Rule};

pub fn make_redis_key(key: &str, endpoint: &str, algorithm: &RateLimiterAlgorithms) -> String {
    // Ex : fixed_window:userid:matched_endpoint_from_rules
    format!("{}:{}:{}", algorithm.to_string(), key, endpoint)
}

pub fn populate_redis_kv_rule_algorithm(
    conn: &mut redis::Connection,
    rules: &Vec<Rule>,
) -> Result<(), RedisError> {
    for rule in rules {
        conn.set(
            format!("rules_to_algorithms:{}", rule.hash.clone()),
            rule.algorithm.to_string(),
        )?
    }
    Ok(())
}
