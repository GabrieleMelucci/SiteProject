use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use regex::Regex;
use lazy_static::lazy_static;

use crate::parser::DictEntry;

lazy_static! {
    static ref NORMALIZE_RE: Regex = Regex::new(r"[^a-zA-Z\u4e00-\u9fff]").unwrap();
}

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub lang: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub query: String,
    pub results: Vec<DictEntry>,
}

pub async fn search(
    Query(params): Query<SearchParams>,
    State((_pool, dict)): State<(crate::DbPool, Arc<Vec<DictEntry>>)>,
) -> Json<SearchResult> {
    let query_lower = params.q.to_lowercase();
    let normalized = NORMALIZE_RE.replace_all(&query_lower, "");
    let mut results = Vec::new();

    for entry in dict.iter() {
        let score = match params.lang.as_deref().unwrap_or("chinese") {
            "chinese" => max_similarity(&normalized, &[
                &entry.simplified,
                &entry.traditional,
                &entry.pinyin
            ]),
            _ => entry.definitions.iter()
                .map(|def| similarity(&normalized, &NORMALIZE_RE.replace_all(def, "")))
                .fold(0.0, f32::max)
        };

        if score > 0.8 {
            results.push(entry.clone());
        }
    }

    Json(SearchResult {
        query: params.q,
        results: results.into_iter().take(15).collect(),
    })
}

fn max_similarity(a: &str, options: &[&str]) -> f32 {
    options.iter()
        .map(|b| similarity(a, b))
        .fold(0.0, f32::max)
}

fn similarity(a: &str, b: &str) -> f32 {
    if a == b {
        1.0
    } else {
        0.0
    }
}