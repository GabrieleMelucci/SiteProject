use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bcrypt::BcryptError;
use crate::data::models::{LoginError, RegisterError, AuthError};
use diesel::result::Error as DieselError;
use tower_sessions::session::Error as SessionError;
use validator::ValidationErrors;
use serde_json::{json, Error as JsonError};

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

impl From<DieselError> for LoginError {
    fn from(err: DieselError) -> Self {
        LoginError::DatabaseError(err)
    }
}

impl From<BcryptError> for LoginError {
    fn from(err: BcryptError) -> Self {
        LoginError::HashingError(err)
    }
}

impl From<SessionError> for LoginError {
    fn from(err: SessionError) -> Self {
        LoginError::SessionError(err.to_string())
    }
}


impl From<BcryptError> for RegisterError {
    fn from(err: BcryptError) -> Self {
        RegisterError::HashingError(err)
    }
}

impl From<SessionError> for RegisterError {
    fn from(err: SessionError) -> Self {
        RegisterError::SessionError(err.to_string())
    }
}

impl From<ValidationErrors> for RegisterError {
    fn from(err: ValidationErrors) -> Self {
        RegisterError::ValidationError(err.to_string())
    }
}

impl From<JsonError> for RegisterError {
    fn from(err: JsonError) -> Self {
        RegisterError::SessionError(err.to_string())
    }
}

// Utility functions
pub async fn set_user_session(
    session: &tower_sessions::Session,
    user_id: i32,
    email: &str,
) -> Result<(), LoginError> {
    session.insert("logged_in", true).await?;
    session.insert("user_id", user_id).await?;
    session.insert("user_email", email).await?;
    Ok(())
}