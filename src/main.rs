use axum::{
    extract::Extension,
    response::{IntoResponse, Redirect},
    routing::{get, get_service, post},
    Router,
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use std::sync::Arc;
use tera::{Tera, Context};
use time::Duration;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

mod auth;
mod login;
mod model;
mod parser;
mod register;
mod schema;
mod search;
mod utils;
mod deck;

type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[tokio::main]
async fn main() {
    // Database configuration
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://site.db".into());

    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create DB pool");

    // Dictionary data loading
    let dict_data = Arc::new(parser::parse_cedict());

    // Templates configuration
    let templates = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Template parsing error: {}", e);
            std::process::exit(1);
        }
    };
    let templates = Arc::new(templates);

    // Sessions configuration
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)))
        .with_secure(false);

    // Deck API router (only needs pool)
    let deck_api_router = Router::new()
        .route("/", get(deck::list_decks))
        .route("/create", post(deck::create_deck))
        .route("/add-word", post(deck::add_word_to_deck))
        .with_state(pool.clone());

    // Search API router (needs both pool and dict_data)
    let search_api_router = Router::new()
        .route("/", get(search::search_api))
        .with_state((pool.clone(), dict_data.clone()));

    // Combined API router
    let api_router = Router::new()
        .nest("/decks", deck_api_router)
        .nest("/search", search_api_router);

    // Auth router
    let auth_router = Router::new()
        .merge(login::auth_router(pool.clone(), templates.clone()))
        .merge(register::auth_router(pool.clone(), templates.clone()))
        .route("/logout", get(handle_logout));

    // Main application router
    let app = Router::new()
        // Static pages
        .route("/", get(home))
        .route("/about", get(about))
        .route("/changelog", get(changelog))
        .route("/privacy-policy", get(privacy_policy))
        .route("/terms-of-use", get(terms_of_use))
        // Search page
        .route("/search", get(search::search_page))
        // Dashboard
        .route("/dashboard", get(dashboard))
        // Auth routes
        .nest("/auth", auth_router)
        // API routes
        .nest("/api", api_router)
        // Static files
        .nest_service("/static", get_service(ServeDir::new("static")))
        // Shared state and layers
        .layer(Extension(templates))
        .layer(session_layer);

    // Start server
    let listener = match TcpListener::bind("127.0.0.1:5000").await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to address: {}", e);
            std::process::exit(1);
        }
    };

    println!("Server running on http://localhost:5000");

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}

// Handlers for static pages
async fn home(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    utils::render_template(&templates, "ZWCD.html", Context::new())
}

async fn dashboard(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    utils::render_template(&templates, "dashboard.html", Context::new())
}

async fn about(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    utils::render_template(&templates, "about.html", Context::new())
}

async fn changelog(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    utils::render_template(&templates, "changelog.html", Context::new())
}

async fn privacy_policy(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    utils::render_template(&templates, "privacy-policy.html", Context::new())
}

async fn terms_of_use(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    utils::render_template(&templates, "terms-of-use.html", Context::new())
}

// Auth handlers
async fn handle_logout(
    session: tower_sessions::Session
) -> Result<Redirect, auth::LoginError> {
    session.delete().await.map_err(|e| {
        log::error!("Failed to delete session: {}", e);
        auth::LoginError::SessionError("Failed to logout".into())
    })?;
    Ok(Redirect::to("/"))
}