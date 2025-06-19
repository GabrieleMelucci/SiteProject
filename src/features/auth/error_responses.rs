use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use crate::data::models::{LoginError, RegisterError, AuthError};

impl IntoResponse for LoginError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            LoginError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            LoginError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            LoginError::HashingError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Hashing error: {}", e),
            ),
            LoginError::SessionError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", e),
            ),
        };

        let body = json!({
            "error": message,
            "status": status.as_u16()
        });

        (status, axum::Json(body)).into_response()
    }
}

impl IntoResponse for RegisterError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            RegisterError::EmailTaken => (StatusCode::CONFLICT, self.to_string()),
            RegisterError::ValidationError(e) => (StatusCode::BAD_REQUEST, e),
            RegisterError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            RegisterError::HashingError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Hashing error: {}", e),
            ),
            RegisterError::SessionError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", e),
            ),
        };

        let body = json!({
            "error": message,
            "status": status.as_u16()
        });

        (status, axum::Json(body)).into_response()
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::Login(e) => e.into_response(),
            AuthError::Register(e) => e.into_response(),
        }
    }
}