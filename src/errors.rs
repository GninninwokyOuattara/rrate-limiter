use std::sync::PoisonError;

use anyhow::anyhow;

use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use matchit::MatchError;
use redis::RedisError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LimiterError {
    #[error("No match found")]
    NoRouteMatch(#[from] MatchError), // TODO: Better error for this, it needs context. a custom error ?

    #[error("Failed to connect to Redis: {0}")]
    RedisError(#[from] RedisError),

    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl IntoResponse for LimiterError {
    fn into_response(self) -> Response<Body> {
        /* let body = match self {
            LimiterError::NoRouteMatch(err) => {
                println!("No match found {:?}", err);
                "No match found"
            }

            LimiterError::Unknown(error) => {
                println!("Error :: {:?}", error);
                "Internal Server Error"
            }
        }; */

        // There will be a better way to handle those error but for the moment a 502 for all errors is accepted.
        println!("Error {:?}", &self.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

impl<T> From<PoisonError<T>> for LimiterError {
    fn from(_err: PoisonError<T>) -> Self {
        let error = anyhow!("Poisonned lock");
        error.into()
    }
}
