use rrl_core::{LimiterTrackingType, RateLimiterAlgorithms};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Pagination {
    pub page: u32,
    pub page_size: u32,
    pub route: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PostedRule {
    pub route: String,
    pub algorithm: RateLimiterAlgorithms,
    pub tracking_type: LimiterTrackingType,
    pub limit: u32,
    pub expiration: u32,
    pub custom_tracking_key: Option<String>,
    pub status: bool,
    pub ttl: u32,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PatchedRule {
    pub route: Option<String>,
    pub algorithm: Option<RateLimiterAlgorithms>,
    pub tracking_type: Option<LimiterTrackingType>,
    pub limit: Option<u32>,
    pub expiration: Option<u32>,
    pub custom_tracking_key: Option<String>,
    pub status: Option<bool>,
    pub ttl: Option<u32>,
}
