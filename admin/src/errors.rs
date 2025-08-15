use axum::{http::StatusCode, response::IntoResponse};
use rrl_core::{tracing, uuid::Uuid};

use thiserror::Error;
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Internal Server Error")]
    DatabaseError(#[from] rrl_core::tokio_postgres::Error),

    #[error("No match found for rule {0}")]
    RuleNotFound(Uuid),

    #[error("Internal Server Error")]
    Error(#[from] Box<dyn std::error::Error>),

    #[error("{0} is not a valid route pattern")]
    InvalidRoutePattern(String), // The provided route is not matchit compatible

    #[error("{0} already exists")]
    AlreadyExistingRoute(String),
    // Route may be invalid
    // route may already exits  those are two different errors.
    #[error(transparent)]
    Others(#[from] anyhow::Error),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        tracing::error!("Error : {:#?}", &self);
        match &self {
            ServiceError::DatabaseError(err) => match err.as_db_error() {
                Some(postgres_error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        postgres_error.message().to_string(),
                    )
                        .into_response();
                }
                None => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response(),
            },

            ServiceError::RuleNotFound(_err) => {
                (StatusCode::NOT_FOUND, self.to_string()).into_response()
            }

            ServiceError::Error(_err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }

            ServiceError::InvalidRoutePattern(_) | ServiceError::AlreadyExistingRoute(_) => {
                (StatusCode::BAD_REQUEST, self.to_string()).into_response()
            }

            ServiceError::Others(_err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}
