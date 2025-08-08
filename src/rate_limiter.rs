use redis::RedisError;

use crate::utils::make_redis_key;

const FIXED_WINDOW: &str = "fw";
const SLIDING_WINDOW_COUNTER: &str = "swc";
const SLIDING_WINDOW_LOG: &str = "swl";
const LEAKY_BUCKET: &str = "lb";
const TOKEN_BUCKET: &str = "tb";

pub struct RateLimiterHeaders {
    limit: u64,     // Maximum number of requests allowed
    remaining: u64, // Number of requests remaining in the current window
    reset: u64,     // Time in seconds until the rate limit resets
    policy: String, // The rate limiting policy used
}

impl RateLimiterHeaders {
    pub fn new(limit: u64, remaining: u64, reset: u64, policy: String) -> Self {
        Self {
            limit,
            remaining,
            reset,
            policy,
        }
    }

    pub fn to_headers(&self) -> axum::http::HeaderMap {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("X-RateLimit-Limit", self.limit.to_string().parse().unwrap());
        headers.insert(
            "X-RateLimit-Remaining",
            self.remaining.to_string().parse().unwrap(),
        );
        headers.insert("X-RateLimit-Reset", self.reset.to_string().parse().unwrap());
        headers.insert("X-RateLimit-Policy", self.policy.parse().unwrap());
        // headers.insert("Retry-After", self.retry_after.to_string().parse().unwrap());
        headers
    }
}

pub enum RateLimiterAlgorithms {
    FixedWindow,
    SlidingWindowCounter,
    SlidingWindowLog,
    TokenBucket,
    LeakyBucket,
}

impl RateLimiterAlgorithms {
    pub fn to_string(&self) -> String {
        match self {
            RateLimiterAlgorithms::FixedWindow => FIXED_WINDOW.to_string(),
            RateLimiterAlgorithms::SlidingWindowCounter => SLIDING_WINDOW_COUNTER.to_string(),
            RateLimiterAlgorithms::SlidingWindowLog => SLIDING_WINDOW_COUNTER.to_string(),
            RateLimiterAlgorithms::TokenBucket => TOKEN_BUCKET.to_string(),
            RateLimiterAlgorithms::LeakyBucket => LEAKY_BUCKET.to_string(),
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            RateLimiterAlgorithms::FixedWindow => FIXED_WINDOW,
            RateLimiterAlgorithms::SlidingWindowCounter => SLIDING_WINDOW_COUNTER,
            RateLimiterAlgorithms::SlidingWindowLog => SLIDING_WINDOW_COUNTER,
            RateLimiterAlgorithms::TokenBucket => TOKEN_BUCKET,
            RateLimiterAlgorithms::LeakyBucket => LEAKY_BUCKET,
        }
    }

    pub fn from_string(s: &str) -> Result<Self, ()> {
        match s {
            FIXED_WINDOW => Ok(RateLimiterAlgorithms::FixedWindow),
            SLIDING_WINDOW_COUNTER => Ok(RateLimiterAlgorithms::SlidingWindowCounter),
            SLIDING_WINDOW_LOG => Ok(RateLimiterAlgorithms::SlidingWindowLog),
            TOKEN_BUCKET => Ok(RateLimiterAlgorithms::TokenBucket),
            LEAKY_BUCKET => Ok(RateLimiterAlgorithms::LeakyBucket),
            _ => Err(()),
        }
    }

    pub fn get_script(&self) -> &'static str {
        match self {
            RateLimiterAlgorithms::FixedWindow => {
                r#"
                local key = KEYS[1]
                local limit = tonumber(ARGV[1])
                local expiration = tonumber(ARGV[2])

    
                if redis.call('EXISTS', key) == 0 then
                    redis.call('SET', key, 0)
                    redis.call('EXPIRE', key, expiration)
                end

                if redis.call('GET', key) + 1 > limit then
                    local remaining = limit - redis.call('GET', key)
                    local reset = redis.call('TTL', key)
                    return {
                        limit,
                        remaining,
                        reset,
                        'Rate limit exceeded.',
                    }

                else
                    redis.call('INCR', key)
                    local remaining = limit - redis.call('GET', key)
                    local reset = redis.call('TTL', key)
                    return {
                        limit,
                        remaining,
                        reset,
                        'Rate limit not exceeded.',
                    }
                end

                "#
            }
            RateLimiterAlgorithms::SlidingWindowCounter => {
                r#"
                local k = KEYS[1]
                local key = k .. ':ss'
                local key_counter = k .. ':counter'
                
                local limit = tonumber(ARGV[1])
                local expiration = tonumber(ARGV[2])
                local now = redis.call('TIME')[1]
                
                redis.call('ZREMRANGEBYSCORE', key, 0, now - expiration)
                local count = redis.call('ZCARD', key)
                redis.call('EXPIRE', key, expiration + 1)

                if count + 1 > limit then
                    redis.call('EXPIRE', key_counter, expiration + 1)
                    local remaining = limit - count
                    local reset = expiration - (now % expiration)
                    return {
                        limit,
                        remaining,
                        reset,
                        'Rate limit exceeded.',
                    }
                else
                    redis.call('ZADD', key, now, now .. ':' .. redis.call('INCR', key_counter))
                    redis.call('EXPIRE', key_counter, expiration + 1)
                    local remaining = limit - count - 1
                    local reset = expiration - (now % expiration)
                    return {
                        limit,
                        remaining,
                        reset,
                        'Rate limit not exceeded.',
                    }
                end
                "#
            }
            RateLimiterAlgorithms::SlidingWindowLog => todo!(),
            RateLimiterAlgorithms::TokenBucket => todo!(),
            RateLimiterAlgorithms::LeakyBucket => todo!(),
        }
    }
}

pub struct RateLimiter {}

impl RateLimiter {
    pub fn check(
        mut redis_connection: &mut redis::Connection,
        tracked_key: &str,
        hashed_route: &str,
        algorithm: RateLimiterAlgorithms,
        limit: u64,
        expiration: u64,
    ) -> Result<(String, RateLimiterHeaders), RedisError> {
        let redis_key = make_redis_key(tracked_key, hashed_route, &algorithm);

        let script = redis::Script::new(algorithm.get_script());

        let result = script
            .key(redis_key)
            .arg(limit)
            .arg(expiration)
            .invoke::<Vec<String>>(&mut redis_connection)?;

        Ok((
            result[3].clone(),
            RateLimiterHeaders::new(
                result[0].parse().unwrap_or_default(),
                result[1].parse().unwrap_or_default(),
                result[2].parse().unwrap_or_default(),
                algorithm.to_string(),
            ),
        ))
    }
}
