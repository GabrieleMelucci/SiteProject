use serde::{Deserialize, Serialize};
use crate::data::models::DictEntry;

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