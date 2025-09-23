use lazy_static::lazy_static;
use redis::{Script, aio::ConnectionManager};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::{errors::LimiterError, utils::make_redis_key};

#[derive(Debug)]
pub struct RateLimiterHeaders {
    pub limit: u64,     // Maximum number of requests allowed
    pub remaining: u64, // Number of requests remaining in the current window
    pub reset: u64,     // Time in seconds until the rate limit resets
    pub policy: String, // The rate limiting policy used
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
}

const FIXED_WINDOW: &str = "fw";
const SLIDING_WINDOW_COUNTER: &str = "swc";
const SLIDING_WINDOW_LOG: &str = "swl";
const LEAKY_BUCKET: &str = "lb";
const TOKEN_BUCKET: &str = "tb";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateLimiterAlgorithms {
    #[serde(alias = "fw")]
    FixedWindow,
    #[serde(alias = "swc")]
    SlidingWindowCounter,
    #[serde(alias = "swl")]
    SlidingWindowLog,
    #[serde(alias = "tb")]
    TokenBucket,
    #[serde(alias = "lb")]
    LeakyBucket,
}

impl RateLimiterAlgorithms {
    pub fn to_string(&self) -> String {
        match self {
            RateLimiterAlgorithms::FixedWindow => FIXED_WINDOW.to_string(),
            RateLimiterAlgorithms::SlidingWindowCounter => SLIDING_WINDOW_COUNTER.to_string(),
            RateLimiterAlgorithms::SlidingWindowLog => SLIDING_WINDOW_LOG.to_string(),
            RateLimiterAlgorithms::TokenBucket => TOKEN_BUCKET.to_string(),
            RateLimiterAlgorithms::LeakyBucket => LEAKY_BUCKET.to_string(),
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
                        '0',
                    }

                else
                    redis.call('INCR', key)
                    local remaining = limit - redis.call('GET', key)
                    local reset = redis.call('TTL', key)
                    return {
                        limit,
                        remaining,
                        reset,
                        '1',
                    }
                end

                "#
            }
            RateLimiterAlgorithms::SlidingWindowLog => {
                r#"
                local k = KEYS[1]
                local key = k .. ':ss'
                local key_counter = k .. ':counter'
                
                local limit = tonumber(ARGV[1])
                local expiration = tonumber(ARGV[2])
                local now = redis.call('TIME')[1]
                
                redis.call('ZREMRANGEBYSCORE', key, 0, now - expiration)
                local count = redis.call('ZCARD', key)
                

                if count + 1 > limit then
                    redis.call('EXPIRE', key, expiration + 1)
                    redis.call('EXPIRE', key_counter, expiration + 1)
                    local oldest_time_and_member = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                    local oldest_time = tonumber(oldest_time_and_member[2])
                    local reset = (oldest_time + expiration) - now
                    
                    return {
                        limit,
                        0,
                        reset,
                        '0',
                    }
                else
                    redis.call('ZADD', key, now, now .. ':' .. redis.call('INCR', key_counter))
                    redis.call('EXPIRE', key, expiration + 1)
                    redis.call('EXPIRE', key_counter, expiration + 1)
                    local oldest_time_and_member = redis.call('ZRANGE', key, 0, 0, 'WITHSCORES')
                    local oldest_time = tonumber(oldest_time_and_member[2])
                    local reset = (oldest_time + expiration) - now
                    local remaining = limit - count - 1
                    
                    return {
                        limit,
                        remaining,
                        reset,
                        '1',
                    }
                end
                "#
            }
            RateLimiterAlgorithms::SlidingWindowCounter => {
                r#"
                local key = KEYS[1]
                local limit = tonumber(ARGV[1])
                local expiration = tonumber(ARGV[2])
                local now = redis.call('TIME')[1]
                local mod_value = expiration * 3 -- we got three buckets of 'expiration' seconds each

                -- verify that the buckets exists
                if redis.call('EXISTS', key) == 0 then
                    redis.call('HMSET', key, '0', '0', '1', '0', '2', '0')
                end
                
                redis.call('EXPIRE', key, expiration + 1)

                local normalized_now = now % mod_value

                local current_bucket = math.floor(normalized_now / expiration)
                local previous_bucket = (current_bucket + 2) % 3
                local next_bucket = (current_bucket + 1) % 3
                
                redis.call('HSET', key, next_bucket, 0) -- reset the counter for the bucket to come.

                local normalized_to_window = normalized_now % expiration
                local percentage_in_bucket = normalized_to_window / expiration
                
                
                local previous_bucket_count = redis.call('HGET', key, previous_bucket)
                local current_bucket_count = redis.call('HGET', key, current_bucket)

                local weight = (1-percentage_in_bucket) * tonumber(previous_bucket_count) + tonumber(current_bucket_count)
                local reset = expiration - (now % expiration)
                if weight > limit then

                    return {
                        limit,
                        0,
                        reset,
                        '0',
                    }
                else
                    redis.call('HINCRBY', key, current_bucket, 1)
                    local remaining = limit - (1-percentage_in_bucket) * tonumber(previous_bucket_count) - tonumber(current_bucket_count) - 1

                    return {
                        limit,
                        remaining,
                        reset,
                        '1',
                    }
                end
                "#
            }
            RateLimiterAlgorithms::TokenBucket => {
                r#"
                local key = KEYS[1]
                local limit = tonumber(ARGV[1])
                local expiration = tonumber(ARGV[2])
                local now = tonumber(redis.call('TIME')[1])
                local drop_rate = limit / expiration


                -- init the tokens bucket
                redis.call('HSETNX', key, 'count', limit)
                redis.call('HSETNX', key, 'last_rq_timestamp', now)
                redis.call('EXPIRE', key, expiration, 'NX')

                local ttl = redis.call('TTL', key)
                
                local elapsed = now - tonumber(redis.call('HGET', key, 'last_rq_timestamp'))
                local bucket_refill_rate = elapsed * drop_rate
                
                local current_count = tonumber(redis.call('HGET', key, 'count'))
                local new_count = math.min(limit, current_count + bucket_refill_rate)

                redis.call('HSET', key, 'count', new_count) 


                if new_count - 1 < 0 then
                    return {
                        limit,
                        0,
                        ttl,
                        '0',
                    }
                else 
                    redis.call('HSET', key, 'count', new_count - 1)
                    redis.call('HSET', key, 'last_rq_timestamp', now)
                    return {
                        limit,
                        new_count - 1,
                        ttl,
                        '1',
                    }
                end
                "#
            }
            RateLimiterAlgorithms::LeakyBucket => {
                r#"
                local key = KEYS[1]
                local limit = tonumber(ARGV[1])
                local expiration = tonumber(ARGV[2])
                local now = tonumber(redis.call('TIME')[1])
                local drop_rate = limit / expiration


                -- init the leaky bucket
                redis.call('HSETNX', key, 'count', 0)
                redis.call('HSETNX', key, 'last_rq_timestamp', now)
                redis.call('EXPIRE', key, expiration, 'NX')

                local ttl = redis.call('TTL', key)
                
                local elapsed = now - tonumber(redis.call('HGET', key, 'last_rq_timestamp'))
                local request_lazily_dropped = elapsed * drop_rate
                
                local current_count = tonumber(redis.call('HGET', key, 'count'))
                local new_count = math.max(0, current_count - request_lazily_dropped)

                redis.call('HSET', key, 'count', new_count) 


                if new_count + 1 > limit then
                    return {
                        limit,
                        0,
                        ttl,
                        '0',
                    }
                else 
                    redis.call('HSET', key, 'count', new_count + 1)
                    redis.call('HSET', key, 'last_rq_timestamp', now)
                    return {
                        limit,
                        limit - math.ceil(new_count) - 1,
                        ttl,
                        '1',
                    }
                end
                "#
            }
        }
    }
}

impl TryFrom<String> for RateLimiterAlgorithms {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            FIXED_WINDOW => Ok(RateLimiterAlgorithms::FixedWindow),
            SLIDING_WINDOW_COUNTER => Ok(RateLimiterAlgorithms::SlidingWindowCounter),
            SLIDING_WINDOW_LOG => Ok(RateLimiterAlgorithms::SlidingWindowLog),
            TOKEN_BUCKET => Ok(RateLimiterAlgorithms::TokenBucket),
            LEAKY_BUCKET => Ok(RateLimiterAlgorithms::LeakyBucket),
            _ => Err(format!("{} is not a valid algorithm.", value)),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LimiterTrackingType {
    #[serde(alias = "ip")]
    IP, // Should be tracked by the ip address of the requester
    #[serde(alias = "header")]
    Header, // A custom header should be tracked
}

impl LimiterTrackingType {
    pub fn to_string(&self) -> String {
        match self {
            LimiterTrackingType::IP => "ip".to_string(),
            LimiterTrackingType::Header => "header".to_string(),
        }
    }
}

impl TryFrom<String> for LimiterTrackingType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "header" => Ok(LimiterTrackingType::Header),
            "ip" => Ok(LimiterTrackingType::IP), // Ip should be the default
            _ => Err(format!("{value} is not a valid tracking type.")),
        }
    }
}

impl From<LimiterTrackingType> for String {
    fn from(value: LimiterTrackingType) -> Self {
        match value {
            LimiterTrackingType::Header => "header".to_string(),
            LimiterTrackingType::IP => "ip".to_string(),
        }
    }
}

lazy_static! {
    static ref SCRIPTS: HashMap<String, Script> = {
        let mut scripts = HashMap::new();
        scripts.insert(
            RateLimiterAlgorithms::FixedWindow.to_string(),
            Script::new(RateLimiterAlgorithms::FixedWindow.get_script()),
        );
        scripts.insert(
            RateLimiterAlgorithms::SlidingWindowCounter.to_string(),
            Script::new(RateLimiterAlgorithms::SlidingWindowCounter.get_script()),
        );
        scripts.insert(
            RateLimiterAlgorithms::SlidingWindowLog.to_string(),
            Script::new(RateLimiterAlgorithms::SlidingWindowLog.get_script()),
        );
        scripts.insert(
            RateLimiterAlgorithms::TokenBucket.to_string(),
            Script::new(RateLimiterAlgorithms::TokenBucket.get_script()),
        );
        scripts.insert(
            RateLimiterAlgorithms::LeakyBucket.to_string(),
            Script::new(RateLimiterAlgorithms::LeakyBucket.get_script()),
        );
        scripts
    };
}

pub async fn execute_rate_limiting(
    mut pool: ConnectionManager,
    tracked_key: &str,
    rule_redis_config_key: &str,
    algorithm: &RateLimiterAlgorithms,
    limit: u64,
    expiration: u64,
    route: &str,
) -> Result<RateLimiterHeaders, LimiterError> {
    tracing::debug!(
        "Executing rate limiting with key {tracked_key}, algorithm {algorithm:?}, limit {limit}, expiration {expiration} and rule_redis_config_key {rule_redis_config_key}"
    );
    let redis_key = make_redis_key(tracked_key, rule_redis_config_key, algorithm);
    let script = SCRIPTS.get(&algorithm.to_string()).unwrap();

    let result: Vec<u64> = script
        .key(redis_key)
        .arg(limit)
        .arg(expiration)
        .invoke_async(&mut pool)
        .await?;

    let headers = RateLimiterHeaders::new(result[0], result[1], result[2], algorithm.to_string());
    tracing::debug!("Resulting headers after rate limiting: {:#?}", headers);

    if result[3] == 0 {
        return Err(LimiterError::RateLimitExceeded {
            headers,
            key: tracked_key.to_string(),
            msg: "Rate limit exceeded".to_string(),
            route: route.to_string(),
        });
    }

    Ok(headers)
}
