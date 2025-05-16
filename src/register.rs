use axum::{
    extract::{Form, State},
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use bcrypt::{hash, DEFAULT_COST};
use diesel::prelude::*;
use std::sync::Arc;
use tera::Tera;
use crate::utils::render_template;

use crate::{
    model::{NewUser, User},
    schema::users::dsl::{email, users},
    DbPool, 
    auth::{AuthError, RegisterForm, set_user_session},
};

pub async fn show_register_form(
    State((_pool, tera)): State<(DbPool, Arc<Tera>)>
) -> Result<Html<String>, AuthError> {
    let mut context = tera::Context::new();
    context.insert("title", "Register");
    Ok(render_template(&tera, "register.html", context))
}

#[axum::debug_handler]
pub async fn handle_register(
    State((pool, _tera)): State<(DbPool, Arc<Tera>)>,
    session: tower_sessions::Session,
    Form(form): Form<RegisterForm>,
) -> Result<Redirect, AuthError> {
    form.validate()?;

    let mut conn = pool.get().map_err(|_| AuthError::SessionError("Failed to get DB connection".into()))?;

    let existing_user = users
        .filter(email.eq(&form.email))
        .first::<User>(&mut conn)
        .optional()?;

    if existing_user.is_some() {
        return Err(AuthError::EmailTaken);
    }

    let hashed_password = hash(&form.password, DEFAULT_COST)?;
    
    diesel::insert_into(users)
        .values(&NewUser {
            email: &form.email,
            password: &hashed_password,
        })
        .execute(&mut conn)?;

    let user = users
        .filter(email.eq(&form.email))
        .first::<User>(&mut conn)?;

    set_user_session(&session, user.user_id, &user.email).await?;

    Ok(Redirect::to("/dashboard"))
}

pub fn auth_router(pool: DbPool, tera: Arc<Tera>) -> Router {
    Router::new()
        .route("/register", get(show_register_form).post(handle_register))
        .with_state((pool, tera))
}