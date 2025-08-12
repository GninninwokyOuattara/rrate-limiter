const FIXED_WINDOW: &str = "fw";
const SLIDING_WINDOW_COUNTER: &str = "swc";
const SLIDING_WINDOW_LOG: &str = "swl";
const LEAKY_BUCKET: &str = "lb";
const TOKEN_BUCKET: &str = "tb";

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
                        'Rate limit exceeded.',
                    }
                else
                    redis.call('HINCRBY', key, current_bucket, 1)
                    local remaining = limit - (1-percentage_in_bucket) * tonumber(previous_bucket_count) - tonumber(current_bucket_count) - 1

                    return {
                        limit,
                        remaining,
                        reset,
                        'Rate limit not exceeded.',
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
                        'Rate limit exceeded.',
                    }
                else 
                    redis.call('HSET', key, 'count', new_count - 1)
                    redis.call('HSET', key, 'last_rq_timestamp', now)
                    return {
                        limit,
                        new_count - 1,
                        ttl,
                        'Rate limit not exceeded.',
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
                        'Rate limit exceeded.',
                    }
                else 
                    redis.call('HSET', key, 'count', new_count + 1)
                    redis.call('HSET', key, 'last_rq_timestamp', now)
                    return {
                        limit,
                        limit - math.ceil(new_count) - 1,
                        ttl,
                        'Rate limit not exceeded.',
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

#[derive(Clone)]
pub enum LimiterTrackingType {
    IP,     // Should be tracked by the ip address of the requester
    Custom, // A custom header should be tracked
}

// impl From<String> for LimiterTrackingType {
//     fn from(value: String) -> Self {
//         match value.as_str() {
//             "custom" => LimiterTrackingType::Custom,
//             _ => LimiterTrackingType::IP, // Ip is the default
//         }
//     }
// }

impl TryFrom<String> for LimiterTrackingType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "custom" => Ok(LimiterTrackingType::Custom),
            "ip" => Ok(LimiterTrackingType::IP), // Ip should be the default
            _ => Err(format!("{value} is not a valid tracking type.")),
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
