use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;
use uuid::Uuid;

use crate::{LimiterTrackingType, RateLimiterAlgorithms};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub id: String,                       // The key to be rate limited
    pub route: String,                    // the endpoint : pattern like route
    pub algorithm: RateLimiterAlgorithms, // The algorithm to use
    pub limit: i32,                       // The maximum number of requests
    pub expiration: i32,                  // The time window for the rate limit
    pub tracking_type: LimiterTrackingType,
    pub custom_tracking_key: Option<String>,
    pub status: bool,
    pub date_creation: chrono::DateTime<Utc>,
    pub date_modification: chrono::DateTime<Utc>,
}

impl TryFrom<Row> for Rule {
    type Error = Box<dyn std::error::Error>;

    fn try_from(value: Row) -> Result<Self, Self::Error> {
        // let algorithm: RateLimiterAlgorithms = value.get::<_, String>("algorithm").try_into()?;

        Ok(Rule {
            id: value.get::<_, Uuid>("id").into(),
            route: value.get("route"),
            algorithm: value.get("algorithm"),
            tracking_type: value.get("tracking_type"),
            limit: value.get("limit"),
            expiration: value.get("expiration"),
            custom_tracking_key: value.get("custom_tracking_key"),
            status: value.get("status"),
            date_creation: value.get("date_creation"),
            date_modification: value.get("date_modification"),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinimalRule {
    pub id: String,
    pub route: String,
}
