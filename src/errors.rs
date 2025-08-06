use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LimiterError {
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

impl IntoResponse for LimiterError {
    fn into_response(self) -> Response<Body> {
        let body = match self {
            LimiterError::Unknown(error) => {
                println!("Error :: {:?}", error);
                "Internal Server Error"
            }
        };

        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}
