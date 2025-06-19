use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::get,
    Router,
};
use std::sync::Arc;
use tera::{Tera, Context};
use log;

use crate::{
    DbPool,
    utils::{set_user_session, render_template},
    data::repositories::UserRepository
};
use crate::data::models::{LoginError, LoginForm};

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
    let mut conn = pool.get()
        .map_err(|e| {
            log::error!("Failed to get DB connection: {}", e);
            LoginError::SessionError("Failed to get DB connection".into())
        })?;
    
    let user = UserRepository::find_by_email(&mut conn, &form.email)
        .map_err(|e| {
            log::error!("Database error during login: {}", e);
            LoginError::DatabaseError(e)
        })?;

    match user {
        Some(user) => {
            let is_valid = UserRepository::verify_password(&user.password, &form.password)
                .map_err(|e| {
                    log::error!("Password verification failed: {}", e);
                    LoginError::HashingError(e)
                })?;
            
            if is_valid {
                set_user_session(&session, user.user_id, &user.email).await?;
                Ok(Redirect::to("/dashboard"))
            } else {
                log::warn!("Invalid password for user: {}", form.email);
                Err(LoginError::InvalidCredentials)
            }
        },
        None => {
            log::warn!("User not found: {}", form.email);
            Err(LoginError::InvalidCredentials)
        }
    }
}

pub fn auth_router(pool: DbPool, tera: Arc<Tera>) -> Router {
    Router::new()
        .route("/login", get(show_login_form).post(handle_login))
        .with_state((pool, tera))
}