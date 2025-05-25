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