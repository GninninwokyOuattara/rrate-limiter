use anyhow::Context;
use axum::http::HeaderMap;
use redis::{AsyncCommands, Commands, RedisError, aio::ConnectionManager};
use rrl_core::{LimiterTrackingType, RateLimiterAlgorithms, Rule, chrono};

use crate::errors;

pub fn make_redis_key(
    key_tracked: &str,
    hashed_route: &str,
    limit_algorithm: &RateLimiterAlgorithms,
) -> String {
    // Ex : fixed_window : id of the matched route : key being tracked for rate limitation
    format!(
        "{}:{}:{}",
        limit_algorithm.to_string(),
        hashed_route,
        key_tracked
    )
}

pub fn _populate_redis_kv_rule_algorithm(
    conn: &mut redis::Connection,
    rules: &Vec<Rule>,
) -> Result<(), RedisError> {
    for rule in rules {
        conn.set(
            format!("rules_to_algorithms:{}", rule.id.clone()),
            rule.algorithm.to_string(),
        )?
    }
    Ok(())
}

pub async fn populate_redis_with_rules(
    mut conn: ConnectionManager,
    rules: &Vec<Rule>,
) -> Result<(), RedisError> {
    for rule in rules {
        // set the integers fields
        let _: () = conn
            .hset_multiple(
                format!("rules:{}", rule.id),
                &[("limit", rule.limit), ("expiration", rule.expiration)],
            )
            .await
            .unwrap();

        // Set the string fields.
        let tracking_type: String = rule.tracking_type.clone().into();
        let _: () = conn
            .hset_multiple(
                format!("rules:{}", rule.id),
                &[
                    ("algorithm", rule.algorithm.to_string()),
                    ("tracking_type", tracking_type),
                    (
                        "custom_tracking_key",
                        rule.custom_tracking_key.clone().unwrap_or("".to_string()),
                    ),
                ],
            )
            .await
            .unwrap();
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
        LimiterTrackingType::Header => {
            let custom_key = custom_header_key.context("Custom header should not be null")?;
            if let Some(key) = headers.get(&custom_key) {
                return Ok(key.to_str().unwrap().to_string());
            } else {
                return Err(errors::LimiterError::TrackedKeyNotFound(custom_key));
            }
        }
    }
}

pub fn generate_dummy_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "user1".to_string(),
            route: "/products".to_string(),

            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 100,
            expiration: 60,
            tracking_type: LimiterTrackingType::Header,
            custom_tracking_key: Some("product_key".to_string()),
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            id: "user2".to_string(),
            route: "/api/v1/orders".to_string(),

            algorithm: RateLimiterAlgorithms::TokenBucket,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::IP,
            custom_tracking_key: None,
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            id: "user2".to_string(),
            route: "/api/v1/commands".to_string(),

            algorithm: RateLimiterAlgorithms::SlidingWindowLog,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::Header,
            custom_tracking_key: Some("x-api-key".to_string()),
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            id: "user3".to_string(),
            route: "/api/v1/users/{id}".to_string(),

            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 200,
            expiration: 300,
            tracking_type: LimiterTrackingType::Header,
            custom_tracking_key: Some("x-api-key".to_string()),
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            // FIXED WINDOW TEST
            id: "user2".to_string(),
            route: "/api/v1/fw".to_string(),

            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::IP,
            custom_tracking_key: None,
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            // SLIDING WINDOW LOG TEST
            id: "user2".to_string(),
            route: "/api/v1/swl".to_string(),

            algorithm: RateLimiterAlgorithms::SlidingWindowLog,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::IP,
            custom_tracking_key: None,
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            // SLIDING WINDOW COUNTER TEST
            id: "user2".to_string(),
            route: "/api/v1/swc".to_string(),

            algorithm: RateLimiterAlgorithms::SlidingWindowCounter,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::IP,
            custom_tracking_key: None,
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            // TOKEN BUCKET TEST
            id: "user2".to_string(),
            route: "/api/v1/tb".to_string(),

            algorithm: RateLimiterAlgorithms::TokenBucket,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::IP,
            custom_tracking_key: None,
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            // LEAKY BUCKET TEST
            id: "user2".to_string(),
            route: "/api/v1/lb".to_string(),

            algorithm: RateLimiterAlgorithms::LeakyBucket,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::IP,
            custom_tracking_key: None,
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
        Rule {
            // Direct
            id: "user2".to_string(),
            route: "/api/v1/direct".to_string(),

            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 50,
            expiration: 5,
            tracking_type: LimiterTrackingType::Header,
            custom_tracking_key: Some("foo".to_string()),
            status: true,
            ttl: 60,
            date_creation: chrono::Utc::now(),
            date_modification: chrono::Utc::now(),
        },
    ]
}
