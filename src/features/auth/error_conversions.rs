use bcrypt::BcryptError;
use diesel::result::Error as DieselError;
use tower_sessions::session::Error as SessionError;
use validator::ValidationErrors;
use serde_json::Error as JsonError;
use crate::data::models::{LoginError, RegisterError};

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