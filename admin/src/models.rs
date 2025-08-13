use rrl_core::{LimiterTrackingType, RateLimiterAlgorithms};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Pagination {
    pub page: i32,
    pub page_size: i32,
    pub route: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostedRule {
    pub route: String,
    pub algorithm: RateLimiterAlgorithms,
    pub tracking_type: LimiterTrackingType,
    pub limit: i32,
    pub expiration: i32,
    pub custom_tracking_key: Option<String>,
    pub status: bool,
    pub ttl: i32,
}
