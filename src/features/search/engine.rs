use crate::data::models::DictEntry;
use lazy_static::lazy_static;
use regex::Regex;
use unidecode::unidecode;

lazy_static! {
    static ref NORMALIZE_RE: Regex = Regex::new(r"[^a-zA-Z\u4e00-\u9fff]").unwrap();
    static ref PUNCTUATION_RE: Regex = Regex::new(r"[.,;:!?]").unwrap();
}

pub struct SearchEngine;

impl SearchEngine {
    pub fn search_entries(
        query: &str,
        dict: &[DictEntry],
        lang: Option<&str>,
    ) -> Vec<(DictEntry, f32)> {
        let query_lower = query.to_lowercase();
        let normalized = NORMALIZE_RE.replace_all(&query_lower, "");
        let mut results = Vec::new();

        for entry in dict {
            let score = match lang.unwrap_or("chinese") {
                "chinese" => SearchEngine::max_similarity(
                    &normalized,
                    &[
                        &entry.simplified,
                        &entry.traditional,
                        &PUNCTUATION_RE.replace_all(&entry.pinyin, "").to_lowercase(),
                        &SearchEngine::remove_tones(&entry.pinyin),
                    ],
                ),
                _ => entry
                    .definitions
                    .iter()
                    .map(|def| {
                        let clean_def = NORMALIZE_RE.replace_all(def, "").to_lowercase();
                        SearchEngine::similarity(&normalized, &clean_def)
                    })
                    .fold(0.0, f32::max),
            };

            if score > 0.8 {
                results.push((entry.clone(), score));
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results
    }

    fn remove_tones(pinyin: &str) -> String {
        unidecode(pinyin)
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect()
    }

    fn max_similarity(a: &str, options: &[&str]) -> f32 {
        options
            .iter()
            .map(|b| SearchEngine::similarity(a, b))
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
}
