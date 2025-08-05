pub struct Rule {
    pub key: String,                      // The key to be rate limited
    pub endpoint: String,                 // the endpoint : simple or regexlike
    pub algorithm: RateLimiterAlgorithms, // The algorithm to use
    pub limit: u64,                       // The maximum number of requests
    pub expiration: u64,                  // The time window for the rate limit
}

pub fn generate_dummy_rules() -> Vec<Rule> {
    vec![
        Rule {
            key: "user1".to_string(),
            endpoint: "/products".to_string(),
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 100,
            expiration: 60,
        },
        Rule {
            key: "user2".to_string(),
            endpoint: "/api/v1/orders".to_string(),
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 50,
            expiration: 120,
        },
        // rule wiht regex
        Rule {
            key: "user3".to_string(),
            endpoint: r"/api/v1/users/\d+".to_string(), // regex-like endpoint
            algorithm: RateLimiterAlgorithms::FixedWindow,
            limit: 200,
            expiration: 300,
        },
    ]
}
