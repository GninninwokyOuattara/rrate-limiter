use axum::{http::StatusCode, response::IntoResponse};
use rrl_core::tokio_postgres;

use anyhow::anyhow;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Internal Server Error")]
    DatabaseError(#[from] rrl_core::tokio_postgres::Error),

    #[error("Internal Server Error")]
    Error(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    Others(#[from] anyhow::Error),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        println!("Error : {:#?}", &self);
        match &self {
            ServiceError::DatabaseError(_err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }

            ServiceError::Error(_err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }

            ServiceError::Others(_err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
            }
        }
    }
}
