use anyhow::{Context, anyhow};
use hyper::HeaderMap;
use matchit::Router;
use matchit::Router as MatchitRouter;
use redis::{
    AsyncCommands, Commands, JsonAsyncCommands, RedisError, Script, aio::ConnectionManager,
};
use serde_json::json;

use std::collections::HashMap;

use crate::{
    errors::{self, LimiterError},
    rate_limiter::{LimiterTrackingType, RateLimiterAlgorithms},
    rules::{MinimalRule, Rule},
};

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

pub async fn _populate_redis_with_rules(
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

            Err(errors::LimiterError::NoIpFound)
        }
        LimiterTrackingType::Header => {
            let custom_key = custom_header_key.context("Custom header should not be null")?;
            if let Some(key) = headers.get(&custom_key) {
                Ok(key.to_str().unwrap().to_string())
            } else {
                Err(errors::LimiterError::TrackedKeyNotFound(custom_key))
            }
        }
    }
}

pub async fn get_rules_from_redis(
    connection: &mut ConnectionManager,
) -> Result<HashMap<String, MinimalRule>, RedisError> {
    // Get all fields and values from redis.
    let res: String = connection.json_get("rules", "$").await?;
    let rules: Vec<HashMap<String, MinimalRule>> = serde_json::from_str(&res)?;
    tracing::info!("length of rules retrieved: {}", rules.len());
    if rules.is_empty() {
        return Ok(HashMap::new());
    }

    let hash = rules.first().unwrap();
    Ok(hash.clone())
}

pub fn instantiate_matcher_with_rules(rules: HashMap<String, MinimalRule>) -> Router<String> {
    let mut matcher = MatchitRouter::new();
    for (rule_id, rule) in rules {
        match matcher.insert(rule.route.clone(), rule_id.clone()) {
            Ok(_) => {
                tracing::debug!(
                    "Successfully inserted route: {} with id: {}",
                    rule.route,
                    rule_id
                );
            }
            Err(e) => {
                tracing::warn!("Failed to insert route: {e}. Errors are ignored.");
            }
        }
    }
    matcher
}

pub async fn get_rules_information_by_redis_json_key(
    redis_connection: &mut ConnectionManager,
    key: &str,
) -> Result<Rule, LimiterError> {
    let res: String = redis_connection
        .json_get("rules", format!("$.{}", key).as_str())
        .await?;
    tracing::debug!("Rules retrieved: {}", &res);
    let rules: Vec<Rule> =
        serde_json::from_str(&res).map_err(|err| errors::LimiterError::Unknown(anyhow!(err)))?;

    Ok(rules
        .first()
        .ok_or(errors::LimiterError::Unknown(anyhow!(
            "No rule found for key: {}",
            key
        )))?
        .clone())
}

pub fn make_rules_configuration_script(rules: Vec<Rule>) -> Script {
    // Empty or initialize the rules hash
    let check_initialization = r"
        redis.call('JSON.SET', 'rules', '$', '{}')

    ";

    // Build the rules, redis call after redis call
    let mut script_rules: Vec<String> = vec![];
    script_rules.push(check_initialization.to_string());

    rules.into_iter().for_each(|rule| {
        let id = rule.id.clone().to_string();
        let rule_json = json!(
            {
                "id": rule.id,
                "route": rule.route,
                "algorithm": rule.algorithm.to_string(),
                "tracking_type": rule.tracking_type.to_string(),
                "limit": rule.limit,
                "expiration": rule.expiration,
                "custom_tracking_key": rule.custom_tracking_key.unwrap_or("".to_string()),
                "active": rule.active.unwrap_or(true).to_string()
            }
        );

        let script = format!(r#"redis.call('JSON.SET', 'rules', '$.{id}' , '{rule_json}')"#);
        script_rules.push(script);
    });

    // Finilize the script by publishing the update
    let publish = "redis.call('PUBLISH', 'rl_update', 'update')".to_string();
    let script_rules = script_rules.join("\n");
    Script::new(&format!("{}\n{}", script_rules, publish))
}
