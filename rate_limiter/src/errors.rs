use std::sync::PoisonError;

use anyhow::anyhow;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use redis::RedisError;
use rrl_core::tracing;
use thiserror::Error;

use crate::rate_limiter::RateLimiterHeaders;

#[derive(Error, Debug)]
pub enum LimiterError {
    #[error("No match found for route {0}")]
    NoRouteMatch(String),

    #[error("Tracked key {0} not found in request headers")]
    TrackedKeyNotFound(String),

    #[error(
        "No IP found in request headers. Are you sure you are using a proxy? looked for [x-forwarded-for, x-real-ip, forwarded]"
    )]
    NoIpFound,

    #[error("Rate limit exceeded for {key} on route {route}")]
    RateLimitExceeded {
        headers: RateLimiterHeaders,
        key: String,
        msg: String,
        route: String,
    },
    #[error("Internal Server Error")]
    RedisError(#[from] RedisError),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl LimiterError {
    pub fn into_hyper_response(self) -> Response<Full<Bytes>> {
        tracing::debug!("Limiter Error : {:#?}", &self,);
        match self {
            LimiterError::NoRouteMatch(msg) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(msg)))
                .unwrap(),
            LimiterError::TrackedKeyNotFound(msg) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(msg)))
                .unwrap(),
            LimiterError::NoIpFound => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(LimiterError::NoIpFound.to_string())))
                .unwrap(),
            LimiterError::RateLimitExceeded {
                headers,
                key: _,
                msg: _,
                route: _,
            } => {
                return Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("limit", headers.limit)
                    .header("remaining", headers.remaining)
                    .header("reset", headers.reset)
                    .header("policy", headers.policy)
                    .body(Full::new(Bytes::from("Rate limit exceeded!")))
                    .unwrap();
            }
            LimiterError::RedisError(_) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Internal Server Error")))
                .unwrap(),
            LimiterError::Unknown(_) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(Bytes::from("Internal Server Error")))
                .unwrap(),
        }
    }
}

impl<T> From<PoisonError<T>> for LimiterError {
    fn from(_err: PoisonError<T>) -> Self {
        let error = anyhow!("Poisonned lock");
        error.into()
    }
}
