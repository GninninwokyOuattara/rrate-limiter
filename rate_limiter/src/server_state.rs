use std::sync::Arc;

use opentelemetry::metrics::Counter;
use parking_lot::RwLock;
use redis::aio::ConnectionManager;

pub struct States {
    pub route_matcher: Arc<RwLock<matchit::Router<String>>>,
    pub pool: ConnectionManager,
    pub rl_total_requests: Counter<u64>,
    pub rl_allowed_requests: Counter<u64>,
    pub rl_rejected_requests: Counter<u64>,
}
