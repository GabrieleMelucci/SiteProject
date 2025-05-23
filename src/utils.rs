use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use tera::{Context, Tera};

pub fn render_template(
    tera: &Tera,
    template_name: &str,
    context: Context,
) -> Result<Html<String>, String> {
    tera.render(template_name, &context)
        .map(Html)
        .map_err(|e| format!("Template rendering error: {}", e))
}

pub async fn is_authenticated(session: &tower_sessions::Session) -> bool {
    session.get::<String>("user_email").await.map_or(false, |email| email.is_some())
}