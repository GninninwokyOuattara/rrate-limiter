use anyhow::anyhow;
use bytes::Bytes;
use std::sync::Arc;

use crate::{
    errors::LimiterError,
    rate_limiter::execute_rate_limiting,
    server_state::States,
    utils::{get_rules_information_by_redis_json_key, get_tracked_key_from_header},
};

use http_body_util::Full;
use hyper::{Request, Response};

pub async fn limiter_handler(
    states: Arc<States>,
    request: Request<hyper::body::Incoming>,
) -> anyhow::Result<Response<Full<Bytes>>, LimiterError> {
    let path = request.uri().path();
    let res = async {
        // Retrieve the key associated with this route using the matcher.
        // That key will be used to index the rule information inside the from the cache.
        let associated_key = states
            .route_matcher
            .clone()
            .read()
            .at(path)
            .map_err(|_err| LimiterError::NoRouteMatch(path.to_string()))?
            .value
            .clone();

        // Retrieve the rule informations from the redis cache.
        let limiter_rule =
            get_rules_information_by_redis_json_key(&mut states.pool.clone(), &associated_key)
                .await?;

        // In case the rule is disabled (active=false)
        if let Some(v) = &limiter_rule.active
            && !(*v)
        {
            let response = Response::builder()
                .body(Full::new(Bytes::from("Rate limit not exceeded.")))
                .map_err(|_err| LimiterError::Unknown(anyhow!("Unable to build response")));

            return response;
        }

        let tracking_key = get_tracked_key_from_header(
            request.headers(),
            &limiter_rule.tracking_type,
            limiter_rule.custom_tracking_key.as_deref(),
        )?;

        let headers = execute_rate_limiting(
            states.pool.clone(),
            &tracking_key,
            &associated_key,
            &limiter_rule.algorithm,
            limiter_rule.limit as u64,
            limiter_rule.expiration as u64,
            path,
        )
        .await?;

        let response = Response::builder()
            .header("limit", headers.limit)
            .header("remaining", headers.remaining)
            .header("reset", headers.reset)
            .header("policy", headers.policy)
            .body(Full::new(Bytes::from("Rate limit not exceeded")))
            .map_err(|_err| LimiterError::Unknown(anyhow!("Unable to build response")));
        response
    };

    return match res.await {
        Ok(res) => Ok(res),
        Err(err) => Ok(err.into_hyper_response()),
    };
}
