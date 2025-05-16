use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::{get},
    Router,
};
use bcrypt::verify;
use diesel::prelude::*;
use std::sync::Arc;
use tera::{Tera, Context};

use crate::{
    schema::users::dsl::{users, email},
    model::User,
    DbPool, 
    auth::{AuthError, LoginForm, set_user_session},
    utils::render_template,
};

pub async fn show_login_form(
    State((_pool, tera)): State<(DbPool, Arc<Tera>)>
) -> Result<Html<String>, AuthError> {
    let mut context = Context::new();
    context.insert("title", "Login");
    Ok(render_template(&tera, "login.html", context))
}

#[axum::debug_handler]
pub async fn handle_login(
    State((pool, _tera)): State<(DbPool, Arc<Tera>)>,
    session: tower_sessions::Session,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, AuthError> {
    let mut conn = pool.get().map_err(|_| AuthError::SessionError("Failed to get DB connection".into()))?;
    
    let user = users
        .filter(email.eq(&form.email))
        .first::<User>(&mut conn)
        .optional()?;

    if let Some(user) = user {
        if verify(&form.password, &user.password)? {
            set_user_session(&session, user.user_id, &user.email).await?;
            return Ok(Redirect::to("/dashboard"));
        }
    }

    Err(AuthError::InvalidCredentials)
}

pub fn auth_router(pool: DbPool, tera: Arc<Tera>) -> Router {
    Router::new()
        .route("/login", get(show_login_form).post(handle_login))
        .with_state((pool, tera))
}