use axum::response::Response;
use axum::{Json, response::IntoResponse};
use thiserror::Error;

use crate::common::dto::ApiResponse;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("An error occurred: {0}")]
    Generic(&'static str),

    #[error("Invalid input: {0}")]
    InvalidInput(&'static str),

    #[error("Database error: {0}")]
    DatabaseError(&'static str),

    #[error("Network error: {0}")]
    NetworkError(&'static str),

    #[error("Authentication failed: {0}")]
    AuthenticationError(&'static str),

    #[error("Authorization failed: {0}")]
    AuthorizationError(&'static str),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            AppError::Generic(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidInput(_) => axum::http::StatusCode::BAD_REQUEST,
            AppError::DatabaseError(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            AppError::NetworkError(_) => axum::http::StatusCode::SERVICE_UNAVAILABLE,
            AppError::AuthenticationError(_) => axum::http::StatusCode::UNAUTHORIZED,
            AppError::AuthorizationError(_) => axum::http::StatusCode::FORBIDDEN,
        };
        let response = Json(ApiResponse::<()>::failure(
            status.as_u16(),
            self.to_string(),
        ));
        (status, response).into_response()
    }
}
