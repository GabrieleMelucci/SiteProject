use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::get,
    Router,
};
use std::sync::Arc;
use tera::{Tera, Context};
use validator::Validate;

use crate::{
    DbPool,
    utils::{set_user_session, render_template},
    data::repositories::UserRepository,
    data::models::{RegisterError, RegisterForm}
};

pub async fn show_register_form(
    State((_pool, tera)): State<(DbPool, Arc<Tera>)>
) -> Result<Html<String>, RegisterError> {
    let mut context = Context::new();
    context.insert("title", "Register");
    Ok(render_template(&tera, "register.html", context))
}

#[axum::debug_handler]
pub async fn handle_register(
    State((pool, _tera)): State<(DbPool, Arc<Tera>)>,
    session: tower_sessions::Session,
    Form(form): Form<RegisterForm>,
) -> Result<Redirect, RegisterError> {
    form.validate().map_err(RegisterError::from)?;

    let mut conn = pool.get()
        .map_err(|e| {
            log::error!("Failed to get DB connection: {}", e);
            RegisterError::SessionError("Failed to get DB connection".into())
        })?;

    if UserRepository::email_exists(&mut conn, &form.email)? {
        log::warn!("Registration attempt with existing email: {}", form.email);
        return Err(RegisterError::EmailTaken);
    }

    let user = UserRepository::create_user(&mut conn, &form.email, &form.password)
        .map_err(|e| {
            log::error!("User creation failed: {}", e);
            RegisterError::DatabaseError(e)
        })?;

    set_user_session(&session, user.user_id, &user.email)
        .await
        .map_err(|e| {
            log::error!("Failed to set session: {:?}", e);
            RegisterError::SessionError("Failed to set user session".into())
        })?;

    log::info!("New user registered: {}", form.email);
    Ok(Redirect::to("/dashboard"))
}

pub fn auth_router(pool: DbPool, tera: Arc<Tera>) -> Router {
    Router::new()
        .route("/register", get(show_register_form).post(handle_register))
        .with_state((pool, tera))
}