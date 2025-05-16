use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bcrypt::BcryptError;
use diesel::result::Error as DieselError;
use serde::{Deserialize};
use serde_json::{Error as JsonError, json};
use thiserror::Error;
use tower_sessions::session::Error as SessionError;
use validator::{Validate, ValidationErrors};

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Email already registered")]
    EmailTaken,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Password too weak")]
    WeakPassword,
    #[error("Invalid email format")]
    InvalidEmail,
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Database error")]
    DatabaseError(#[from] DieselError),
    #[error("Hashing error")]
    HashingError(#[from] BcryptError),
    #[error("Session error: {0}")]
    SessionError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::EmailTaken => (StatusCode::CONFLICT, self.to_string()),
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::WeakPassword => (
                StatusCode::BAD_REQUEST,
                "Password must be at least 8 characters".to_string(),
            ),
            AuthError::InvalidEmail => (StatusCode::BAD_REQUEST, self.to_string()),
            AuthError::ValidationError(e) => (StatusCode::BAD_REQUEST, e),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };
        
        let body = json!({
            "error": message,
            "status": status.as_u16()
        });
        
        (status, axum::Json(body)).into_response()
    }
}

impl From<SessionError> for AuthError {
    fn from(err: SessionError) -> Self {
        AuthError::SessionError(err.to_string())
    }
}

impl From<JsonError> for AuthError {
    fn from(err: JsonError) -> Self {
        AuthError::SessionError(err.to_string())
    }
}

impl From<ValidationErrors> for AuthError {
    fn from(err: ValidationErrors) -> Self {
        AuthError::ValidationError(err.to_string())
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterForm {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

pub async fn set_user_session(
    session: &tower_sessions::Session,
    user_id: i32,
    email: &str,
) -> Result<(), AuthError> {
    session.insert("user_id", user_id).await?;
    session.insert("user_email", email).await?;
    Ok(())
}