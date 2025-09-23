use std::sync::Arc;

use parking_lot::RwLock;
use redis::aio::ConnectionManager;

pub struct States {
    pub route_matcher: Arc<RwLock<matchit::Router<String>>>,
    pub pool: ConnectionManager,
}
