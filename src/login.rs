use axum::{
    extract::{Form, State},
    response::{IntoResponse, Redirect, Html},
};
use bcrypt::{hash, verify, DEFAULT_COST};
use diesel::prelude::*;
use tera::Tera;
use std::sync::Arc;
use tower_sessions::Session;
use thiserror::Error;

use crate::{
    schema::users::dsl::{users, username},
    model::{User, NewUser},
    DbPool,
};

#[derive(Error, Debug)]
pub enum LoginError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Database error")]
    DatabaseError(#[from] diesel::result::Error),
    #[error("Hashing error")]
    HashingError(#[from] bcrypt::BcryptError),
    #[error("Session error: {0}")]
    SessionError(String),
}

impl IntoResponse for LoginError {
    fn into_response(self) -> axum::response::Response {
        match self {
            LoginError::InvalidCredentials => (
                axum::http::StatusCode::UNAUTHORIZED,
                "Invalid credentials",
            ),
            _ => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
            ),
        }
        .into_response()
    }
}

impl From<tower_sessions::session::Error> for LoginError {
    fn from(err: tower_sessions::session::Error) -> Self {
        LoginError::SessionError(err.to_string())
    }
}

impl From<serde_json::Error> for LoginError {
    fn from(err: serde_json::Error) -> Self {
        LoginError::SessionError(err.to_string())
    }
}

#[derive(serde::Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[axum::debug_handler]
pub async fn login(
    State((pool, _)): State<(DbPool, Arc<Vec<crate::parser::DictEntry>>)>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, LoginError> {
    let mut conn = pool.get().unwrap();
    let user = users
        .filter(username.eq(&form.username))
        .first::<User>(&mut conn)
        .optional()?;

    if let Some(user) = user {
        if verify(&form.password, &user.password)? {
            session.insert("user_id", user.id).await?;
            return Ok(Redirect::to("/"));
        }
    }

    Err(LoginError::InvalidCredentials)
}

pub fn render_template(
    tera: &Tera,
    template_name: &str,
    context: Option<tera::Context>,
) -> Html<String> {
    let ctx = context.unwrap_or_default();
    Html(tera.render(template_name, &ctx).unwrap())
}

#[axum::debug_handler]
pub async fn register(
    State((pool, _)): State<(DbPool, Arc<Vec<crate::parser::DictEntry>>)>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, LoginError> {
    let mut conn = pool.get().unwrap();
    let hashed_password = hash(&form.password, DEFAULT_COST)?;

    diesel::insert_into(users)
        .values(&NewUser {
            username: &form.username,
            password: &hashed_password,
        })
        .execute(&mut conn)?;

    Ok(Redirect::to("/auth/login"))
}

#[axum::debug_handler]
pub async fn logout(session: Session) -> Result<Redirect, LoginError> {
    session.flush().await?;
    Ok(Redirect::to("/"))
}
