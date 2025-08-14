use axum::{http::StatusCode, response::IntoResponse};
use rrl_core::uuid::Uuid;

use thiserror::Error;
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Internal Server Error")]
    DatabaseError(#[from] rrl_core::tokio_postgres::Error),

    #[error("No match found for rule {0}")]
    RuleNotFound(Uuid),

    #[error("Internal Server Error")]
    Error(#[from] Box<dyn std::error::Error>),

    #[error(transparent)]
    Others(#[from] anyhow::Error),
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        println!("Error : {:#?}", &self);
        match &self {
            ServiceError::DatabaseError(err) => {
                // if let Some(err) = _err.as_db_error() {
                //     return (StatusCode::BAD_REQUEST, err.message()).into_response();
                // };

                // return (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response();
                match err.as_db_error() {
                    Some(postgres_error) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            postgres_error.message().to_string(),
                        )
                            .into_response();
                    }
                    None => {
                        return (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
                            .into_response();
                    }
                };
            }

            ServiceError::RuleNotFound(_err) => {
                (StatusCode::NOT_FOUND, self.to_string()).into_response()
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
