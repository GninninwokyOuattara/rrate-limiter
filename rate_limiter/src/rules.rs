use opentelemetry::KeyValue;
use redis::Connection;
use serde::{Deserialize, Serialize, de};
use std::collections::HashMap;
use uuid::Uuid;

use crate::rate_limiter::{LimiterTrackingType, RateLimiterAlgorithms};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub id: String,                       // The key to be rate limited
    pub route: String,                    // the endpoint : pattern like route
    pub algorithm: RateLimiterAlgorithms, // The algorithm to use
    pub limit: i32,                       // The maximum number of requests
    pub expiration: i32,                  // The time window for the rate limit
    pub tracking_type: LimiterTrackingType,
    pub custom_tracking_key: Option<String>,
    #[serde(deserialize_with = "redis_deserialize_bool")]
    pub active: Option<bool>,
}

impl Rule {
    pub fn new(
        route: String,
        algorithm: RateLimiterAlgorithms,
        limit: i32,
        expiration: i32,
        tracking_type: LimiterTrackingType,
        custom_tracking_key: Option<String>,
        active: Option<bool>,
    ) -> Self {
        if tracking_type.to_string() == "header"
            && (custom_tracking_key.is_none() || custom_tracking_key.clone().unwrap().is_empty())
        {
            panic!(
                "Custom tracking key is required when tracking type is header. Route: {}",
                route
            );
        }

        Rule {
            id: Uuid::new_v4().to_string(),
            route,
            algorithm,
            limit,
            expiration,
            tracking_type,
            custom_tracking_key,
            active: active.or(Some(true)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinimalRule {
    pub id: String,
    pub route: String,
}

fn redis_deserialize_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: &str = de::Deserialize::deserialize(deserializer)?;

    match s {
        "true" => Ok(Some(true)),
        "false" => Ok(Some(false)),
        _ => Ok(None),
    }
}

pub fn get_rules_route_and_id(
    connection: &mut Connection,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Get all fields and values from redis.
    let maybe_response: Option<String> = redis::cmd("JSON.GET")
        .arg("rules")
        .arg("$..route")
        .arg("$..id")
        .query(connection)?;

    tracing::debug!("Response from redis: {:?}", &maybe_response);

    let Some(response) = maybe_response else {
        return Ok(HashMap::default());
    };

    tracing::debug!("rules and id query response :: {:#?}", &response);

    let rules: HashMap<String, Vec<String>> =
        serde_json::from_str(&response).expect("Failed to parse rules into valid JSON.");

    let length = rules
        .get(&"$..route".to_string())
        .expect("Route keys not found")
        .len();
    tracing::debug!("length of rules keys: {}", length);

    let mut route_to_id: HashMap<String, String> = HashMap::new();
    for i in 0..length {
        let route = rules.get(&"$..route".to_string()).unwrap().get(i).unwrap();

        let id = rules
            .get(&"$..id".to_string())
            .expect("Failed to get id key")
            .get(i)
            .unwrap_or_else(|| panic!("Failed to get id key at index {}", i));
        route_to_id.insert(route.clone(), id.clone());
    }
    tracing::debug!("route_to_id: {:#?}", route_to_id);

    Ok(route_to_id)
}

impl From<Rule> for Vec<KeyValue> {
    fn from(value: Rule) -> Self {
        vec![
            KeyValue::new("id", value.id),
            KeyValue::new("name", value.route),
            KeyValue::new("algorithm", value.algorithm.to_string()),
            KeyValue::new("limit", value.limit as i64),
            KeyValue::new("expiration", value.expiration as i64),
            KeyValue::new("tracking_type", value.tracking_type.to_string()),
            KeyValue::new(
                "custom_tracking_key",
                value.custom_tracking_key.unwrap_or_default(),
            ),
        ]
    }
}
