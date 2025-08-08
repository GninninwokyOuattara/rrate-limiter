use anyhow::Context;
use axum::http::HeaderMap;
use redis::{Commands, RedisError};

use crate::{
    errors,
    rate_limiter::RateLimiterAlgorithms,
    rules::{LimiterTrackingType, Rule},
};

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
        // set the integers fields
        let _: () = conn.hset_multiple(
            format!("rules:{}", rule.hash),
            &[("limit", rule.limit), ("expiration", rule.expiration)],
        )?;

        // Set the string fields.
        let tracking_type: String = rule.tracking_type.clone().into();
        let _: () = conn.hset_multiple(
            format!("rules:{}", rule.hash),
            &[
                ("algorithm", rule.algorithm.to_string()),
                ("tracking_type", tracking_type),
                (
                    "custom_tracking_key",
                    rule.custom_tracking_key.clone().unwrap_or("".to_string()),
                ),
            ],
        )?;
    }
    Ok(())
}

const STANDARD_IP_HEADERS: [&str; 3] = ["x-forwarded-for", "x-real-ip", "forwarded"];

/// Retrieves the value of the tracked key  from the request headers based on the specified tracking type.
///
/// # Arguments
///
/// * `headers` - The HTTP headers from which the tracked key is to be extracted.
/// * `tracking_type` - The type of tracking to be used, either by IP address or using a custom header.
/// * `custom_header_key` - An optional custom header key to be used when `tracking_type` is `LimiterTrackingType::Custom`.
///
/// # Returns
///
/// * `Ok(String)` - Returns the tracked key as a string if successful.
/// * `Err(LimiterError::TrackedKeyNotFound)` - Returns an error if the tracked key cannot be found in the headers.

pub fn get_tracked_key_from_header(
    headers: &HeaderMap,
    tracking_type: &LimiterTrackingType,
    custom_header_key: Option<String>,
) -> Result<String, errors::LimiterError> {
    match tracking_type {
        LimiterTrackingType::IP => {
            for key in STANDARD_IP_HEADERS {
                if let Some(ip) = headers.get(key) {
                    return Ok(ip.to_str().unwrap().to_string());
                }
            }

            return Err(errors::LimiterError::TrackedKeyNotFound("".to_string()));
        }
        LimiterTrackingType::Custom => {
            let custom_key = custom_header_key.context("Custom header should not be null")?;
            if let Some(key) = headers.get(&custom_key) {
                return Ok(key.to_str().unwrap().to_string());
            } else {
                return Err(errors::LimiterError::TrackedKeyNotFound(custom_key));
            }
        }
    }
}
