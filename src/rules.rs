use crate::rate_limiter::RateLimiterAlgorithms;

pub struct Rule {
    pub key: String,                      // The key to be rate limited
    pub route: String,                    // the endpoint : pattern like route
    pub hash: String,                     // The hash of the pattern, used as key in cache.
    pub algorithm: RateLimiterAlgorithms, // The algorithm to use
    pub limit: u64,                       // The maximum number of requests
    pub expiration: u64,                  // The time window for the rate limit
}

pub fn generate_dummy_rules() -> Vec<Rule> {
    vec![
        Rule {
            key: "user1".to_string(),
            route: "/products".to_string(),
            hash: "product_key".to_string(),
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 100,
            expiration: 60,
        },
        Rule {
            key: "user2".to_string(),
            route: "/api/v1/orders".to_string(),
            hash: "api_key".to_string(),
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 50,
            expiration: 120,
        },
        Rule {
            key: "user3".to_string(),
            route: "/api/v1/users/{id}".to_string(),
            hash: "user_key".to_string(),
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 200,
            expiration: 300,
        },
    ]
}
