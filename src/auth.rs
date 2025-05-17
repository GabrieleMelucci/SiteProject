use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bcrypt::BcryptError;
use diesel::result::Error as DieselError;
use serde::Deserialize;
use serde_json::{json, Error as JsonError};
use thiserror::Error;
use tower_sessions::session::Error as SessionError;
use validator::{Validate, ValidationErrors};

// Errori specifici per il login
#[derive(Error, Debug)]
pub enum LoginError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Database error")]
    DatabaseError(DieselError),
    #[error("Hashing error")]
    HashingError(BcryptError),
    #[error("Session error: {0}")]
    SessionError(String),
}

// Errori specifici per la registrazione
#[derive(Error, Debug)]
pub enum RegisterError {
    #[error("Email already registered")]
    EmailTaken,
    #[error("Password too weak")]
    ValidationError(String),
    #[error("Database error")]
    DatabaseError(#[from] DieselError),
    #[error("Hashing error")]
    HashingError(BcryptError),
    #[error("Session error: {0}")]
    SessionError(String),
}

// Enum principale che unisce tutti gli errori
#[derive(Error, Debug)]
pub enum AuthError {
    #[error(transparent)]
    Login(#[from] LoginError),
    #[error(transparent)]
    Register(#[from] RegisterError),
}

// Implementazione di IntoResponse per LoginError
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

// Implementazione di IntoResponse per RegisterError
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

// Implementazione di IntoResponse per AuthError
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

// Form structs
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

// Utility functions
pub async fn set_user_session(
    session: &tower_sessions::Session,
    user_id: i32,
    email: &str,
) -> Result<(), LoginError> {
    session.insert("user_id", user_id).await?;
    session.insert("user_email", email).await?;
    Ok(())
}