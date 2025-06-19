use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Extension, Json,
};
use std::sync::Arc;
use tera::Context;

use crate::{
    data::models::*,
    utils::{self, render_template},
    features::search::SearchEngine
};
use crate::data::models::DictEntry;

// Handler for HTML page
pub async fn search_page(
    Extension(templates): Extension<Arc<tera::Tera>>,
    session: tower_sessions::Session
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("query", "");
    context.insert("logged_in", &utils::is_logged_in(&session).await);
    context.insert("user_id", &utils::get_current_user_id(&session).await);
    render_template(&templates, "search.html", context)
}

// Slimmed-down API handler
pub async fn search_api(
    Query(params): Query<SearchParams>,
    State((_pool, dict)): State<(crate::DbPool, Arc<Vec<DictEntry>>)>
) -> Json<SearchResult> {
    let results = SearchEngine::search_entries(&params.q, &dict, params.lang.as_deref())
        .into_iter()
        .take(15)
        .map(|(entry, _)| entry)
        .collect();

    Json(SearchResult {
        query: params.q,
        results,
    })
}