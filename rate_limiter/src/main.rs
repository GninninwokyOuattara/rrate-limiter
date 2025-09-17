use anyhow::anyhow;
use bytes::Bytes;
use hyper_util::rt::TokioIo;
use parking_lot::RwLock;
use rrl_core::{tracing, tracing_subscriber};
use std::sync::Arc;

use redis::aio::ConnectionManager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    errors::LimiterError,
    rate_limiter::execute_rate_limiting,
    utils::{
        get_rules_from_redis, get_rules_information_by_redis_json_key, get_tracked_key_from_header,
        instantiate_matcher_with_rules,
    },
};

use std::net::SocketAddr;

use http_body_util::Full;
use hyper::{Request, Response};
use hyper::{server::conn::http1, service::service_fn};
use tokio::net::TcpListener;

mod errors;
mod rate_limiter;
mod utils;

struct States {
    route_matcher: Arc<RwLock<matchit::Router<String>>>,
    pool: ConnectionManager,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis_host = std::env::var("RL_REDIS_HOST").unwrap_or("localhost".to_string());
    let redis_port = std::env::var("RL_REDIS_PORT").unwrap_or("6379".to_string());

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("connecting to redis...");
    let client = redis::Client::open(format!(
        "redis://{}:{}?protocol=resp3",
        redis_host, redis_port
    ))?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let config = redis::aio::ConnectionManagerConfig::new()
        .set_push_sender(tx)
        .set_automatic_resubscription();
    let mut redis_connection = client.get_connection_manager_with_config(config).await?;
    let mut con_for_task = redis_connection.clone();
    tracing::info!("Managed connection to redis established.");
    redis_connection.subscribe("rl_update").await.unwrap(); // We actually want to fails if it is impossible to subscribe initially.
    tracing::info!(
        "Subscribed to rl_update channel. Updates will trigger a rebuild of the matcher."
    );

    let rules_config = get_rules_from_redis(&mut redis_connection).await.unwrap();
    let route_matcher = Arc::new(RwLock::new(instantiate_matcher_with_rules(rules_config))); // Initial instance of the matcher.
    let route_matcher_for_task = Arc::clone(&route_matcher);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            tracing::info!("Event received: {msg:?}");
            let new_rules = get_rules_from_redis(&mut con_for_task).await.unwrap();
            let length = new_rules.len();
            let new_router = instantiate_matcher_with_rules(new_rules);
            *route_matcher_for_task.write() = new_router;
            tracing::info!("Matcher has been rebuilt with {length} routes.");
        }
    });

    let states = Arc::new(States {
        route_matcher: route_matcher.clone(),
        pool: redis_connection,
    });

    tracing::info!("Starting server on port 3000");

    let addr: SocketAddr = ([0, 0, 0, 0], 3000).into();
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let states_in_loop = states.clone();

        tokio::spawn(async move {
            let _ = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| {
                        let cloned_states_for_limiter = states_in_loop.clone();
                        limiter_handler(cloned_states_for_limiter, req)
                    }),
                )
                .await;
        });
    }
}

async fn limiter_handler(
    states: Arc<States>,
    request: Request<hyper::body::Incoming>,
) -> anyhow::Result<Response<Full<Bytes>>, LimiterError> {
    let res = async {
        // Retrieve the key associated with this route using the matcher.
        // That key will be used to index the rule information inside the from the cache.
        let associated_key = states
            .route_matcher
            .clone()
            .read()
            .at(request.uri().path())
            .map_err(|_err| LimiterError::NoRouteMatch(request.uri().path().to_string()))?
            .value
            .clone();

        // Retrieve the rule informations from the redis cache.
        let limiter_rule =
            get_rules_information_by_redis_json_key(&mut states.pool.clone(), &associated_key)
                .await?;

        // In case the rule is disabled (active=false)
        if let Some(v) = &limiter_rule.active
            && *v == false
        {
            let response = Response::builder()
                .body(Full::new(Bytes::from("Rate limit not exceeded.")))
                .map_err(|_err| LimiterError::Unknown(anyhow!("Unable to build response")));

            return response;
        }

        let tracking_key = get_tracked_key_from_header(
            request.headers(),
            &limiter_rule.tracking_type,
            limiter_rule.custom_tracking_key,
        )?;

        let headers = execute_rate_limiting(
            states.pool.clone(),
            &tracking_key,
            &associated_key,
            limiter_rule.algorithm,
            limiter_rule.limit as u64,
            limiter_rule.expiration as u64,
        )
        .await?;

        let response = Response::builder()
            .header("limit", headers.limit)
            .header("remaining", headers.remaining)
            .header("reset", headers.reset)
            .header("policy", headers.policy)
            .body(Full::new(Bytes::from("Rate limit not exceeded")))
            .map_err(|_err| LimiterError::Unknown(anyhow!("Unable to build response")));
        return response;
    };

    return match res.await {
        Ok(res) => Ok(res),
        Err(err) => Ok(err.into_hyper_response()),
    };
}
