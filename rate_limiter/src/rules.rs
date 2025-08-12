use rrl_core::{LimiterTrackingType, RateLimiterAlgorithms, Rule};

pub fn generate_dummy_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "user1".to_string(),
            route: "/products".to_string(),

            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 100,
            expiration: 60,
            tracking_type: LimiterTrackingType::Custom,
            custom_tracking_key: Some("product_key".to_string()),
            status: true,
            ttl: 60,
            date_creation: 1223244,
            date_modification: 1344555,
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
            date_creation: 1223244,
            date_modification: 1344555,
        },
        Rule {
            id: "user2".to_string(),
            route: "/api/v1/commands".to_string(),

            algorithm: RateLimiterAlgorithms::SlidingWindowLog,
            limit: 50,
            expiration: 120,
            tracking_type: LimiterTrackingType::Custom,
            custom_tracking_key: Some("x-api-key".to_string()),
            status: true,
            ttl: 60,
            date_creation: 1223244,
            date_modification: 1344555,
        },
        Rule {
            id: "user3".to_string(),
            route: "/api/v1/users/{id}".to_string(),

            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 200,
            expiration: 300,
            tracking_type: LimiterTrackingType::Custom,
            custom_tracking_key: Some("x-api-key".to_string()),
            status: true,
            ttl: 60,
            date_creation: 1223244,
            date_modification: 1344555,
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
            date_creation: 1223244,
            date_modification: 1344555,
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
            date_creation: 1223244,
            date_modification: 1344555,
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
            date_creation: 1223244,
            date_modification: 1344555,
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
            date_creation: 1223244,
            date_modification: 1344555,
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
            date_creation: 1223244,
            date_modification: 1344555,
        },
        Rule {
            // Direct
            id: "user2".to_string(),
            route: "/api/v1/direct".to_string(),

            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 50,
            expiration: 5,
            tracking_type: LimiterTrackingType::Custom,
            custom_tracking_key: Some("foo".to_string()),
            status: true,
            ttl: 60,
            date_creation: 1223244,
            date_modification: 1344555,
        },
    ]
}
