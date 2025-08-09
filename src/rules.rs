use crate::rate_limiter::RateLimiterAlgorithms;

#[derive(Clone)]
pub enum LimiterTrackingType {
    IP,     // Should be tracked by the ip address of the requester
    Custom, // A custom header should be tracked
}

impl From<String> for LimiterTrackingType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "custom" => LimiterTrackingType::Custom,
            _ => LimiterTrackingType::IP, // Ip is the default
        }
    }
}

impl From<LimiterTrackingType> for String {
    fn from(value: LimiterTrackingType) -> Self {
        match value {
            LimiterTrackingType::Custom => "custom".to_string(),
            LimiterTrackingType::IP => "ip".to_string(),
        }
    }
}

pub struct Rule {
    pub id: String,                       // The key to be rate limited
    pub route: String,                    // the endpoint : pattern like route
    pub hash: String,                     // The hash of the pattern, used as key in cache.
    pub algorithm: RateLimiterAlgorithms, // The algorithm to use
    pub limit: u64,                       // The maximum number of requests
    pub expiration: u64,                  // The time window for the rate limit
    pub tracking_type: LimiterTrackingType,
    pub custom_tracking_key: Option<String>,
}

// TODO: I'll need an implementation from the response of a database.

pub fn generate_dummy_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "user1".to_string(),
            route: "/products".to_string(),
            hash: "445022216b8783f3a2fff1af63def96e".to_string(),
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 100,
            expiration: 60,
            tracking_type: LimiterTrackingType::Custom,
            custom_tracking_key: Some("product_key".to_string()),
        },
        Rule {
            id: "user2".to_string(),
            route: "/api/v1/orders".to_string(),
            hash: "2ba810480dabb4007ddb8108a0ef8d55".to_string(),
            algorithm: RateLimiterAlgorithms::LeakyBucket,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::IP,
            custom_tracking_key: None,
        },
        Rule {
            id: "user2".to_string(),
            route: "/api/v1/commands".to_string(),
            hash: "2ba810480dabb4007ddb8108a0ef8d56".to_string(),
            algorithm: RateLimiterAlgorithms::SlidingWindowLog,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::Custom,
            custom_tracking_key: Some("x-api-key".to_string()),
        },
        Rule {
            id: "user3".to_string(),
            route: "/api/v1/users/{id}".to_string(),
            hash: "dd0855d5107f37a3d4d817e9d931c7d4".to_string(),
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 200,
            expiration: 300,
            tracking_type: LimiterTrackingType::Custom,
            custom_tracking_key: Some("x-api-key".to_string()),
        },
    ]
}
