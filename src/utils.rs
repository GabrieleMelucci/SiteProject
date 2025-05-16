use axum::response::Html;
use tera::{Tera, Context};

pub fn render_template(tera: &Tera, template_name: &str, context: Context) -> Html<String> {
    Html(
        tera.render(template_name, &context)
            .unwrap_or_else(|_| format!("Error rendering template: {}", template_name))
    )
}