use axum::{
    extract::{Query, State},
    response::{IntoResponse},
    Extension,
    Json,
};
use crate::data::models::{SearchParams, SearchResult};
use std::sync::Arc;
use regex::Regex;
use lazy_static::lazy_static;
use unidecode::unidecode;
use tera::Context;

use crate::{data::models::DictEntry, utils::{self, render_template}};

lazy_static! {
    static ref NORMALIZE_RE: Regex = Regex::new(r"[^a-zA-Z\u4e00-\u9fff]").unwrap();
    static ref PUNCTUATION_RE: Regex = Regex::new(r"[.,;:!?]").unwrap();
}

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

// Handler for API endpoint
pub async fn search_api(
    Query(params): Query<SearchParams>,
    State((_pool, dict)): State<(crate::DbPool, Arc<Vec<DictEntry>>)>
) -> Json<SearchResult> {
    let query_lower = params.q.to_lowercase();
    let normalized = NORMALIZE_RE.replace_all(&query_lower, "");
    let mut results = Vec::new();

    for entry in dict.iter() {
        let score = match params.lang.as_deref().unwrap_or("chinese") {
            "chinese" => max_similarity(&normalized, &[
                &entry.simplified,
                &entry.traditional,
                &PUNCTUATION_RE.replace_all(&entry.pinyin, "").to_lowercase(),
                &remove_tones(&entry.pinyin),
            ]),
            _ => entry.definitions.iter()
                .map(|def| {
                    let clean_def = NORMALIZE_RE.replace_all(def, "").to_lowercase();
                    similarity(&normalized, &clean_def)
                })
                .fold(0.0, f32::max)
        };

        if score > 0.8 {
            results.push((entry.clone(), score));
        }
    }

    // Sort by score descending
    results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let top_results: Vec<DictEntry> = results.into_iter()
        .take(15)
        .map(|(entry, _)| entry)
        .collect();

    Json(SearchResult {
        query: params.q,
        results: top_results,
    })
}

fn remove_tones(pinyin: &str) -> String {
    unidecode(pinyin)
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect()
}

fn max_similarity(a: &str, options: &[&str]) -> f32 {
    options.iter()
        .map(|b| similarity(a, b))
        .fold(0.0, f32::max)
}

fn similarity(a: &str, b: &str) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    if a == b {
        return 1.0;
    }

    // Check for partial matches with higher weight
    if b.contains(a) {
        let ratio = a.len() as f32 / b.len() as f32;
        return 0.6 + (ratio * 0.4);
    }

    // Check for reverse partial match
    if a.contains(b) {
        let ratio = b.len() as f32 / a.len() as f32;
        return 0.5 + (ratio * 0.3); 
    }

    // Calculate Jaro-Winkler similarity for better partial matching
    let jaro_winkler = strsim::jaro_winkler(a, b);
    if jaro_winkler > 0.85 {
        return jaro_winkler as f32;
    }

    // Length-based similarity as fallback
    let len_sim = 1.0 - ((a.len() as f32 - b.len() as f32).abs() / (a.len() + b.len()) as f32);
    len_sim * 0.3
}