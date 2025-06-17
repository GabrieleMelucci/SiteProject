use axum::{
    Router,
    extract::{Extension, Path},
    response::{IntoResponse, Redirect, Html},
    http::StatusCode,
    routing::{delete, get, get_service, post},
};
use diesel::{
    SqliteConnection,
    r2d2::{ConnectionManager, Pool},
};
use std::sync::Arc;
use tera::Tera;
use time::Duration;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, MemoryStore, SessionManagerLayer};

mod auth;
mod deck;
mod login;
mod parser;
mod register;
mod schema;
mod search;
mod spaced_repetition_system;
mod user;
mod utils;

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
    let template_path = format!("{}/src/templates/**/*.html", env!("CARGO_MANIFEST_DIR"));
    let templates = Tera::new(&template_path).unwrap_or_else(|e| {
        eprintln!("Template parsing error: {}", e);
        std::process::exit(1);
    });
    let templates = Arc::new(templates);
    
    // Sessions configuration
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)))
        .with_secure(false);

    // Build routers
    let deck_api_router = Router::new()
        .route("/", get(deck::list_decks))
        .route("/{deck_id}", delete(deck::delete_deck))
        .route("/{deck_id}/{word_id}", delete(deck::delete_word_from_deck))
        .route("/words", get(deck::get_deck_words))
        .route("/create", post(deck::create_deck))
        .route("/add-word", post(deck::add_word_to_deck))
        .route("/{deck_id}/study", get(deck::start_study_session))
        .route("/due", get(deck::get_all_due_words))
        .route("/due-count", get(deck::get_due_words_count))
        .route("/{deck_id}/words/{word_id}/review", post(deck::record_word_review))
        .route("/{deck_id}", get(deck::view_deck))
        .with_state(pool.clone())
        .layer(session_layer.clone());

    let search_api_router = Router::new()
        .route("/", get(search::search_api))
        .with_state((pool.clone(), dict_data.clone()))
        .layer(session_layer.clone());

    let api_router = Router::new()
        .nest("/decks", deck_api_router)
        .nest("/search", search_api_router)
        .layer(session_layer.clone());

    let auth_router = Router::new()
        .merge(login::auth_router(pool.clone(), templates.clone()))
        .merge(register::auth_router(pool.clone(), templates.clone()))
        .route("/logout", get(handle_logout))
        .layer(session_layer.clone());

    // Main application router
    let app = Router::new()
        // Root route with auth-aware redirection
        .route("/", get(root_handler))
        // Static pages
        .route("/about", get(about))
        .route("/changelog", get(changelog))
        .route("/privacy-policy", get(privacy_policy))
        .route("/terms-of-use", get(terms_of_use))
        // Search page
        .route("/search", get(search::search_page))
        // Dashboard
        .route("/dashboard", get(dashboard))
        // Decks management
        .route("/decks", get(decks_management))
        .route("/deck/{deck_id}", get(deck_view_page))
        .route("/decks/study", get(due_reviews_page))
        .route("/deck/{deck_id}/study", get(study_page))
        // Auth routes
        .nest("/auth", auth_router)
        // API routes
        .nest("/api", api_router)
        // Static files
        .nest_service("/static", get_service(ServeDir::new("src/static")))
        // Shared state and layers
        .layer(Extension(templates))
        .layer(session_layer);

    // Start server
    let listener = TcpListener::bind("127.0.0.1:5000")
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to address: {}", e);
            std::process::exit(1);
        });

    println!("Server running on http://localhost:5000");

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    });
}

// Handlers
async fn root_handler(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    if utils::is_logged_in(&session).await {
        let mut context = tera::Context::new();
        context.insert("logged_in", &utils::is_logged_in(&session).await);
        utils::render_template(&templates, "dashboard.html", context).into_response()
    } else {
        let mut context = tera::Context::new();
        context.insert("logged_in", &utils::is_logged_in(&session).await);
        utils::render_template(&templates, "ZWCD.html", context).into_response()
    }
}

async fn dashboard(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    utils::render_template(&templates, "dashboard.html", context).into_response()
}

async fn decks_management(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    let logged_in = utils::is_logged_in(&session).await;
    context.insert("logged_in", &logged_in);

    if logged_in {
        if let Some(user_id) = utils::get_current_user_id(&session).await {
            context.insert("user_id", &user_id);
        }
    }

    utils::render_template(&templates, "decks_management.html", context).into_response()
}

async fn about(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    utils::render_template(&templates, "about.html", context).into_response()
}

async fn changelog(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    utils::render_template(&templates, "changelog.html", context).into_response()
}

async fn privacy_policy(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    utils::render_template(&templates, "privacy-policy.html", context).into_response()
}

async fn terms_of_use(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    utils::render_template(&templates, "terms-of-use.html", context).into_response()
}

async fn handle_logout(session: tower_sessions::Session) -> Result<Redirect, auth::LoginError> {
    session.delete().await.map_err(|e| {
        log::error!("Failed to delete session: {}", e);
        auth::LoginError::SessionError("Failed to logout".into())
    })?;
    Ok(Redirect::to("/"))
}

async fn deck_view_page(
    Path(deck_id): Path<i32>,
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    context.insert("deck_id", &deck_id);
    utils::render_template(&templates, "view-deck.html", context).into_response()
}

// Handler for studying a specific deck
pub async fn study_page(
    Path(deck_id): Path<i32>,
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    context.insert("deck_id", &deck_id);

    match templates.render("study-deck.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Template rendering error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
        }
    }
}

// Handler for due reviews (no deck ID)
pub async fn due_reviews_page(
    Extension(templates): Extension<Arc<Tera>>,
    session: tower_sessions::Session,
) -> impl IntoResponse {
    let mut context = tera::Context::new();
    let logged_in = utils::is_logged_in(&session).await;
    context.insert("logged_in", &logged_in);

    if !logged_in {
        return Redirect::to("/auth/login").into_response();
    }

    match templates.render("study-deck.html", &context) {
        Ok(html) => Html(html).into_response(),
        Err(e) => {
            eprintln!("Template rendering error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
        }
    }
}