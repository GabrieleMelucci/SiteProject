use std::fs;
use crate::data::models::DictEntry;

pub fn parse_cedict() -> Vec<DictEntry> {

    let content = fs::read_to_string("src/data/cedict_ts.u8")
        .expect("Errore nella lettura del file cedict_ts.u8");

    let mut entries = Vec::new();

    for line in content.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let parts: Vec<_> = line.split(" /").collect();
        if parts.len() < 2 {
            continue;
        }

        let chars_pinyin: Vec<_> = parts[0].split('[').collect();
        let chars: Vec<_> = chars_pinyin[0].split_whitespace().collect();

        entries.push(DictEntry {
            traditional: chars[0].to_string(),
            simplified: chars[1].to_string(),
            pinyin: chars_pinyin[1].trim_end_matches(']').to_string(),
            definitions: parts[1..].iter()
                .flat_map(|s| s.split('/'))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect(),
        });
    }

    entries
}