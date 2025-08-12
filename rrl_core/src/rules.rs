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
    pub algorithm: RateLimiterAlgorithms, // The algorithm to use
    pub limit: u64,                       // The maximum number of requests
    pub expiration: u64,                  // The time window for the rate limit
    pub tracking_type: LimiterTrackingType,
    pub custom_tracking_key: Option<String>,
    pub status: bool,
    pub ttl: u64,
    pub date_creation: u64,
    pub date_modification: u64,
}
