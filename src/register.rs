use axum::{
    Router,
    extract::{Form, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
};
use bcrypt::{DEFAULT_COST, hash, verify};
use diesel::prelude::*;
use serde::Deserialize;
use std::sync::Arc;
use tera::Tera;
use thiserror::Error;
use tower_sessions::{MemoryStore, Session};
use validator::Validate;

use crate::{
    DbPool,
    model::{NewUser, User},
    schema::users::dsl::{email, users},
};

// Error types
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Email already registered")]
    EmailTaken,
    #[error("Password too weak")]
    WeakPassword,
    #[error("Passwords do not match")]
    PasswordMismatch,
    #[error("Invalid email format")]
    InvalidEmail,
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Database error")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Hashing error")]
    HashingError(#[from] bcrypt::BcryptError),
    #[error("Session error: {0}")]
    SessionError(String),
}

async fn register(form: web::Form<RegistrationForm>) -> impl Responder {
    if form.password != form.repeatPassword {
        return HttpResponse::BadRequest().body("Passwords do not match!");
    }

    // Proceed with registration...
    HttpResponse::Ok().body("Registration successful!")
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AuthError::EmailTaken => (StatusCode::CONFLICT, "Email already registered"),
            AuthError::WeakPassword => (
                StatusCode::BAD_REQUEST,
                "Password must be at least 8 characters",
            ),
            AuthError::PasswordMismatch => (StatusCode::BAD_REQUEST, "Passwords do not match"),
            AuthError::InvalidEmail => (StatusCode::BAD_REQUEST, "Invalid email format"),
            AuthError::ValidationError(e) => (StatusCode::BAD_REQUEST, e),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };

        (status, message).into_response()
    }
}

impl From<tower_sessions::session::Error> for AuthError {
    fn from(err: tower_sessions::session::Error) -> Self {
        AuthError::SessionError(err.to_string())
    }
}

impl From<serde_json::Error> for AuthError {
    fn from(err: serde_json::Error) -> Self {
        AuthError::SessionError(err.to_string())
    }
}

impl From<validator::ValidationErrors> for AuthError {
    fn from(err: validator::ValidationErrors) -> Self {
        AuthError::ValidationError(err.to_string())
    }
}

// Form data
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterForm {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(must_match(other = "password", message = "Passwords do not match"))]
    pub repeatPassword: String,
}

// Handler functions
pub async fn show_register_form(State(tera): State<Arc<Tera>>) -> Result<Html<String>, AuthError> {
    let mut context = tera::Context::new();
    context.insert("title", "Register");
    render_template(&tera, "register.html", Some(context))
}

#[axum::debug_handler]
pub async fn handle_register(
    State((pool, tera)): State<(DbPool, Arc<Tera>)>,
    session: Session,
    Form(form): Form<RegisterForm>,
) -> Result<Redirect, AuthError> {
    // Validate input
    form.validate()?;

    let mut conn = pool
        .get()
        .map_err(|_| AuthError::SessionError("Failed to get DB connection".into()))?;

    // Check if email exists in transaction
    let existing_user = conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let email_taken = users
            .filter(email.eq(&form.email))
            .first::<User>(conn)
            .optional()?;

        Ok(email_taken)
    })?;

    if existing_user.is_some() {
        return Err(AuthError::EmailTaken);
    }

    // Hash password
    let hashed_password = hash(&form.password, DEFAULT_COST)?;

    // Create new user
    diesel::insert_into(users)
        .values(&NewUser {
            email: &form.email,
            password: &hashed_password,
        })
        .execute(&mut conn)?;

    // Set user session
    session
        .insert("user_id", form.email)
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    Ok(Redirect::to("/dashboard"))
}

pub async fn logout(session: Session) -> Result<Redirect, AuthError> {
    session.flush().await?;
    Ok(Redirect::to("/"))
}

// Template rendering
pub fn render_template(
    tera: &Tera,
    template_name: &str,
    context: Option<tera::Context>,
) -> Result<Html<String>, AuthError> {
    let ctx = context.unwrap_or_default();
    tera.render(template_name, &ctx)
        .map(Html)
        .map_err(|e| AuthError::SessionError(e.to_string()))
}

// Router setup
pub fn auth_router(pool: DbPool, tera: Arc<Tera>) -> Router {
    Router::new()
        .route("/register", get(show_register_form).post(handle_register))
        .route("/logout", get(logout))
        .with_state((pool, tera))
}
