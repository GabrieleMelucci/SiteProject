use bcrypt::BcryptError;
use diesel::result::Error as DieselError;
use thiserror::Error;
use validator::Validate;
use serde::Deserialize;

// Login specific errors
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

// Registration specific errors
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

#[derive(Error, Debug)]
pub enum AuthError {
    #[error(transparent)]
    Login(#[from] LoginError),
    #[error(transparent)]
    Register(#[from] RegisterError),
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