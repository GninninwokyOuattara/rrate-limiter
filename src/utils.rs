use redis::{Commands, RedisError};

use crate::{rate_limiter::RateLimiterAlgorithms, rules::Rule};

pub fn make_redis_key(
    key_tracked: &str,
    hashed_route: &str,
    limit_algorithm: &RateLimiterAlgorithms,
) -> String {
    // Ex : fixed_window : hash of the matched route : key being tracked for rate limitation
    format!(
        "{}:{}:{}",
        limit_algorithm.to_string(),
        hashed_route,
        key_tracked
    )
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

pub fn populate_redis_with_rules(
    conn: &mut redis::Connection,
    rules: &Vec<Rule>,
) -> Result<(), RedisError> {
    for rule in rules {
        let _: () = conn.hset_multiple(
            format!("rules:{}", rule.hash),
            &[("limit", rule.limit), ("expiration", rule.expiration)],
        )?;
        let _: () = conn.hset(
            format!("rules:{}", rule.hash),
            "algorithm",
            rule.algorithm.to_string(),
        )?;
    }
    Ok(())
}
