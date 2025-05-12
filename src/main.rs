use axum::{
    routing::{get, post},
    Router,
    response::{IntoResponse, Html},
    extract::Extension,
};
use tower_sessions::{SessionManagerLayer, MemoryStore, Expiry};
use diesel::{SqliteConnection, r2d2::{ConnectionManager, Pool}};
use tera::Tera;
use std::sync::Arc;
use tokio::net::TcpListener;
use time::Duration;

mod login;
mod search;
mod model;
mod parser;
mod schema;

type DbPool = Pool<ConnectionManager<SqliteConnection>>;

#[tokio::main]
async fn main() {
    // Configurazione database
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://site.db".into());
    
    let manager = ConnectionManager::<SqliteConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create DB pool");

    // Caricamento dati dizionario
    let dict_data = Arc::new(parser::parse_cedict());

    // Configurazione templates
    let templates = match Tera::new("templates/**/*.html") {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Template parsing error: {}", e);
            std::process::exit(1);
        }
    };
    let templates = Arc::new(templates);

    // Configurazione sessioni
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(1)))
        .with_secure(false);

    // Creazione router
    let app = Router::new()
        // Route statiche
        .route("/", get(home))
        .route("/about", get(about))
        .route("/changelog", get(changelog))
        .route("/privacy-policy", get(privacy_policy))
        .route("/terms-of-use", get(terms_of_use))
        
        // Route autenticazione
        .route("/auth/register", post(login::register))
        .route("/auth/login", post(login::login))
        .route("/auth/logout", get(login::logout))
        
        // Route API
        .route("/api/search", get(search::search))
        
        // Stato condiviso
        .with_state((pool, dict_data))
        .layer(Extension(templates))
        .layer(session_layer);

    // Avvio server
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

// Handler per le pagine statiche
async fn home(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    login::render_template(&templates, "sitechinese.html", None)
}

async fn about(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    login::render_template(&templates, "about.html", None)
}

async fn changelog(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    login::render_template(&templates, "changelog.html", None)
}

async fn privacy_policy(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    login::render_template(&templates, "privacy-policy.html", None)
}

async fn terms_of_use(Extension(templates): Extension<Arc<Tera>>) -> impl IntoResponse {
    login::render_template(&templates, "terms-of-use.html", None)
}