use axum::response::Html;
use tera::{Tera, Context};

pub fn render_template(tera: &Tera, template_name: &str, context: Context) -> Html<String> {
    Html(
        tera.render(template_name, &context)
            .unwrap_or_else(|_| format!("Error rendering template: {}", template_name))
    )
}

pub async fn is_logged_in(session: &tower_sessions::Session) -> bool {
    session.get::<i32>("user_id").await.unwrap_or(None).is_some()
}

pub async fn get_current_user_id(session: &tower_sessions::Session) -> Option<i32> {
    // First check if the user is logged in
    if !is_logged_in(session).await {
        return None;
    }

    // Try to get the user_id from the session
    match session.get::<i32>("user_id").await {
        Ok(Some(user_id)) => Some(user_id),
        Ok(None) => {
            log::warn!("Session has logged_in=true but no user_id");
            None
        },
        Err(e) => {
            log::error!("Failed to get user_id from session: {}", e);
            None
        }
    }
}