use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::get,
    Router,
};
use bcrypt::verify;
use diesel::prelude::*;
use std::sync::Arc;
use tera::{Tera, Context};
use log;

use crate::{
    schema::users::dsl::{users, email},
    model::User,
    DbPool,
    auth::{LoginError, LoginForm, set_user_session},  
    utils::render_template,
};

pub async fn show_login_form(
    State((_pool, tera)): State<(DbPool, Arc<Tera>)>
) -> Result<Html<String>, LoginError> {
    let mut context = Context::new();
    context.insert("title", "Login");
    Ok(render_template(&tera, "login.html", context))
}

#[axum::debug_handler]
pub async fn handle_login(
    State((pool, _tera)): State<(DbPool, Arc<Tera>)>,
    session: tower_sessions::Session,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, LoginError> {
    let mut conn = pool.get().map_err(|e| {
        log::error!("Failed to get DB connection: {}", e);
        LoginError::SessionError("Failed to get DB connection".into())
    })?;
    
    let user = users
        .filter(email.eq(&form.email))
        .first::<User>(&mut conn)
        .optional()
        .map_err(|e| {
            log::error!("Database error during login: {}", e);
            LoginError::DatabaseError(e)
        })?;

    if let Some(user) = user {
        match verify(&form.password, &user.password) {
            Ok(true) => {
                set_user_session(&session, user.user_id, &user.email).await?;
                Ok(Redirect::to("/dashboard"))
            },
            Ok(false) => {
                log::warn!("Invalid password for user: {}", form.email);
                Err(LoginError::InvalidCredentials)
            },
            Err(e) => {
                log::error!("Password verification failed: {}", e);
                Err(LoginError::HashingError(e))
            }
        }
    } else {
        log::warn!("User not found: {}", form.email);
        Err(LoginError::InvalidCredentials)
    }
}

pub fn auth_router(pool: DbPool, tera: Arc<Tera>) -> Router {
    Router::new()
        .route("/login", get(show_login_form).post(handle_login))
        .with_state((pool, tera))
}