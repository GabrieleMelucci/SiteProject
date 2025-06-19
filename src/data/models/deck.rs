use diesel::Queryable;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Represents a word that belongs to a deck
#[derive(Serialize)]
pub struct DeckWord {
    pub id: i32,              // Auto-generated word ID
    pub simplified: String,   // Simplified Chinese characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traditional: Option<String>, // Traditional Chinese characters (optional)
    pub pinyin: String,       // Pinyin pronunciation
    pub definition: String,   // English definition(s)
    pub deck_id: i32,         // Deck ID this word belongs to
}

/// Represents a deck with all its words
#[derive(Serialize)]
pub struct DeckWithWords {
    pub id: i32,            // Deck ID
    pub name: String,       // Deck name
    pub words: Vec<DeckWord>, // Words in this deck
}

/// Request payload for deck-specific operations
#[derive(Deserialize)]
pub struct DeckId {
    pub deck_id: i32,      // Deck ID for operations
}

/// Basic deck information
#[derive(Serialize)]
pub struct Deck {
    pub id: i32,          // Deck ID
    pub name: String,     // Deck name
    pub privacy_value: bool, // Privacy setting
}

/// Request payload for creating a new deck
#[derive(Deserialize)]
pub struct CreateDeckRequest {
    pub name: String,                // Name for the new deck
    pub word_data: Option<serde_json::Value>, // Optional initial word data
    #[serde(default)]
    pub privacy_value: bool, 
}

/// Request payload for adding a word to a deck
#[derive(Deserialize)]
pub struct AddWordRequest {
    pub deck_id: i32,               // Deck ID to add word to
    pub word_data: serde_json::Value, // Word data (simplified, pinyin, etc.)
}

/// Standard API response format
#[derive(Serialize)]
pub struct ApiResponse {
    pub success: bool,    // Operation status
    pub message: String,  // Result message
}

/// Word representation from database
#[derive(Serialize, Queryable)]
pub struct Word {
    pub id: i32,              // Auto-generated word ID
    pub simplified: String,   // Simplified Chinese
    pub traditional: Option<String>, // Traditional Chinese
    pub pinyin: String,       // Pinyin pronunciation
    pub definition: String,   // English definition(s)
}

#[derive(Serialize)]
pub struct StudyWord {
    pub word: DeckWord,
    pub is_new: bool,          // Whether this is a new word (no prior reviews)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_performance: Option<i32>, // Last performance rating (1-5) if reviewed before
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_review: Option<NaiveDateTime>, // When this word is next due for review
}

#[derive(Deserialize)]
pub struct ReviewRequest {
    pub performance: i32,
}

#[derive(Deserialize)]
pub struct UpdatePrivacyRequest {
    pub privacy_value: bool,
}
