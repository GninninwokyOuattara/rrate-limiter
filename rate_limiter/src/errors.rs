use std::sync::PoisonError;

use anyhow::anyhow;

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use redis::RedisError;
use rrl_core::tracing;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LimiterError {
    #[error("No match found for route {0}")]
    NoRouteMatch(String),

    #[error("Tracked key {0} not found in request headers")]
    TrackedKeyNotFound(String),

    #[error("Internal Server Error")]
    RedisError(#[from] RedisError),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

// TODO: Improve the errors to include more context. just a string is a pain to debug.

impl IntoResponse for LimiterError {
    fn into_response(self) -> Response<Body> {
        tracing::error!("Error : {:#?}", &self);
        let response = match &self {
            LimiterError::RedisError(_err) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            LimiterError::NoRouteMatch(_err) => (StatusCode::OK, self.to_string()),
            LimiterError::Unknown(_error) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            LimiterError::TrackedKeyNotFound(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };
        response.into_response()
    }
}

impl<T> From<PoisonError<T>> for LimiterError {
    fn from(_err: PoisonError<T>) -> Self {
        let error = anyhow!("Poisonned lock");
        error.into()
    }
}
