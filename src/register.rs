use axum::{
    extract::{Form, State},
    response::{Html, Redirect},
    routing::get,
    Router,
};
use bcrypt::{hash, DEFAULT_COST};
use diesel::prelude::*;
use std::sync::Arc;
use tera::{Tera, Context};
use validator::Validate;

use crate::{
    utils::set_user_session,
    data::models::{NewUser, User},
    schema::users::dsl::{email, users},
    DbPool,
    data::models::{RegisterError, RegisterForm},
    utils::render_template,
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
    // Validation with validator
    form.validate().map_err(RegisterError::from)?;

    let mut conn = pool.get().map_err(|e| {
        log::error!("Failed to get DB connection: {}", e);
        RegisterError::SessionError("Failed to get DB connection".into())
    })?;

    // Checks if email is already taken
    let existing_user = users
        .filter(email.eq(&form.email))
        .first::<User>(&mut conn)
        .optional()
        .map_err(|e| {
            log::error!("Database error during registration: {}", e);
            RegisterError::DatabaseError(e)
        })?;

    if existing_user.is_some() {
        log::warn!("Registration attempt with existing email: {}", form.email);
        return Err(RegisterError::EmailTaken);
    }

    // Password hashing
    let hashed_password = hash(&form.password, DEFAULT_COST).map_err(|e| {
        log::error!("Password hashing failed: {}", e);
        RegisterError::HashingError(e)
    })?;

    // Create the new user
    diesel::insert_into(users)
        .values(&NewUser {
            email: &form.email,
            password: &hashed_password,
        })
        .execute(&mut conn)
        .map_err(|e| {
            log::error!("Failed to create user: {}", e);
            RegisterError::DatabaseError(e)
        })?;

    // Fetch the new user
    let user = users
        .filter(email.eq(&form.email))
        .first::<User>(&mut conn)
        .map_err(|e| {
            log::error!("Failed to fetch new user: {}", e);
            RegisterError::DatabaseError(e)
        })?;

    // Set session
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