use std::sync::PoisonError;

use anyhow::anyhow;
use anyhow::bail;
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
    NoRouteMatch(#[from] MatchError),

    /* #[error("redis error")]
    RedisError(#[from] RedisError), */
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
    fn from(err: PoisonError<T>) -> Self {
        let error = anyhow!("Poisonned lock");
        error.into()
    }
}
