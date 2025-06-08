use axum::extract::Path;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use diesel::prelude::*;
use diesel::sql_types::Integer;
use serde::{Deserialize, Serialize};

use crate::{
    DbPool,
    schema::{deck_words, decks, words},
    utils,
};

/// Represents a word that belongs to a deck
#[derive(Serialize)]
pub struct DeckWord {
    pub id: i32,              // Auto-generated word ID
    pub simplified: String,   // Simplified Chinese characters
    pub traditional: Option<String>, // Traditional Chinese characters (optional)
    pub pinyin: String,       // Pinyin pronunciation
    pub definition: String,   // English definition(s)
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
}

/// Request payload for creating a new deck
#[derive(Deserialize)]
pub struct CreateDeckRequest {
    pub name: String,                // Name for the new deck
    pub word_data: Option<serde_json::Value>, // Optional initial word data
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

/// Lists all decks for the current user
pub async fn list_decks(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
) -> Result<Json<Vec<Deck>>, (StatusCode, String)> {
    // Get current user ID from session
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    // Get database connection
    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Query all decks belonging to this user
    let decks = decks::table
        .filter(decks::user_id.eq(user_id))
        .select((decks::deck_id, decks::deck_name))
        .load::<(i32, String)>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?
        .into_iter()
        .map(|(id, name)| Deck { id, name })
        .collect();

    Ok(Json(decks))
}

/// Creates a new deck with optional initial word
pub async fn create_deck(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Json(payload): Json<CreateDeckRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    // Verify user is logged in
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Use transaction to ensure atomicity
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // Create the new deck
        diesel::insert_into(decks::table)
            .values((
                decks::deck_name.eq(&payload.name),
                decks::user_id.eq(user_id),
            ))
            .execute(conn)?;

        // Get the auto-generated deck ID
        let deck_id = diesel::select(diesel::dsl::sql::<Integer>("last_insert_rowid()"))
            .get_result::<i32>(conn)?;

        // Add initial word if provided
        if let Some(word_data) = payload.word_data {
            add_word_to_deck_internal(conn, deck_id, word_data)?;
        }

        Ok(())
    })
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Deck created successfully".to_string(),
    }))
}

/// Adds a word to an existing deck
pub async fn add_word_to_deck(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Json(payload): Json<AddWordRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    // Verify user is logged in
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Verify the user owns the deck they're trying to modify
    let deck_owner: i32 = decks::table
        .filter(decks::deck_id.eq(payload.deck_id))
        .select(decks::user_id)
        .first(&mut conn)
        .map_err(|e| match e {
            diesel::result::Error::NotFound => {
                (StatusCode::NOT_FOUND, "Deck not found".to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)),
        })?;

    if deck_owner != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    // Add the word to the deck
    add_word_to_deck_internal(&mut conn, payload.deck_id, payload.word_data)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Word added to deck successfully".to_string(),
    }))
}

/// Internal function to handle word addition to a deck
/// Returns the auto-generated word ID
fn add_word_to_deck_internal(
    conn: &mut SqliteConnection,
    deck_id: i32,
    word_data: serde_json::Value,
) -> Result<i32, diesel::result::Error> {
    // Extract and validate required word fields
    let simplified = word_data.get("simplified")
        .and_then(|v| v.as_str())
        .ok_or_else(|| diesel::result::Error::DeserializationError(
            "Missing simplified field".into()
        ))?;
    
    let traditional = word_data.get("traditional")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    
    let pinyin = word_data.get("pinyin")
        .and_then(|v| v.as_str())
        .ok_or_else(|| diesel::result::Error::DeserializationError(
            "Missing pinyin field".into()
        ))?;
    
    // Handle definitions which might be an array or a single string
    let definition = match word_data.get("definitions") {
        Some(serde_json::Value::Array(arr)) => {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<&str>>()
                .join(", ")
        },
        Some(serde_json::Value::String(s)) => s.clone(),
        _ => return Err(diesel::result::Error::DeserializationError(
            "Missing or invalid definitions field".into()
        )),
    };

    // Insert the new word (ID will be auto-generated)
    diesel::insert_into(words::table)
        .values((
            words::simplified.eq(simplified),
            words::traditional.eq(traditional),
            words::pinyin.eq(pinyin),
            words::definition.eq(definition),
        ))
        .execute(conn)?;

    // Get the last inserted row id (SQLite specific)
    let word_id = diesel::select(diesel::dsl::sql::<Integer>("last_insert_rowid()"))
        .get_result::<i32>(conn)?;

    // Add to the deck_words junction table
    diesel::insert_into(deck_words::table)
        .values((
            deck_words::deck_id.eq(deck_id),
            deck_words::word_id.eq(word_id),
        ))
        .on_conflict((deck_words::deck_id, deck_words::word_id))
        .do_nothing()
        .execute(conn)?;

    Ok(word_id)
}

/// Deletes a deck and all its words
pub async fn delete_deck(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Path(deck_id): Path<i32>,
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    // Verify user is logged in
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Verify the deck exists and belongs to this user
    let deck_exists = decks::table
        .filter(decks::deck_id.eq(deck_id))
        .filter(decks::user_id.eq(user_id))
        .count()
        .get_result::<i64>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })? > 0;

    if !deck_exists {
        return Err((StatusCode::NOT_FOUND, "Deck not found".to_string()));
    }

    // Use transaction to atomically delete deck and its words
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // First delete from junction table
        diesel::delete(deck_words::table.filter(deck_words::deck_id.eq(deck_id)))
            .execute(conn)?;
        // Then delete the deck itself
        diesel::delete(decks::table.filter(decks::deck_id.eq(deck_id)))
            .execute(conn)
    })
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Deck deleted successfully".to_string(),
    }))
}

/// Views a deck with all its words
pub async fn view_deck(
    Path(deck_id): Path<i32>,
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
) -> Result<Json<DeckWithWords>, (StatusCode, String)> {
    // Verify user is logged in
    let user_id = utils::get_current_user_id(&session)
        .await
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Not logged in".to_string()))?;

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Get basic deck info
    let (id, name): (i32, String) = decks::table
        .filter(decks::deck_id.eq(deck_id))
        .filter(decks::user_id.eq(user_id))
        .select((decks::deck_id, decks::deck_name))
        .first(&mut conn)
        .map_err(|_| {
            (StatusCode::NOT_FOUND, "Deck not found or access denied".to_string())
        })?;

    // Get all words in this deck
    let words = deck_words::table
        .filter(deck_words::deck_id.eq(deck_id))
        .inner_join(words::table)
        .select((
            words::word_id,
            words::simplified,
            words::traditional,
            words::pinyin,
            words::definition,
        ))
        .load::<(i32, String, Option<String>, String, String)>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?
        .into_iter()
        .map(|(id, simplified, traditional, pinyin, definition)| DeckWord {
            id,
            simplified,
            traditional,
            pinyin,
            definition,
        })
        .collect();

    Ok(Json(DeckWithWords { id, name, words }))
}

/// Gets all words in a specific deck
pub async fn get_deck_words(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Json(payload): Json<DeckId>,
) -> Result<Json<Vec<Word>>, (StatusCode, String)> {
    // Verify user is logged in
    let user_id = utils::get_current_user_id(&session)
        .await
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Not logged in".to_string()))?;

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Verify deck ownership
    let deck_owner: i32 = decks::table
        .filter(decks::deck_id.eq(payload.deck_id))
        .select(decks::user_id)
        .first(&mut conn)
        .map_err(|_| {
            (StatusCode::FORBIDDEN, "Deck not found or access denied".to_string())
        })?;

    if deck_owner != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    // Get all words in the deck
    let words = deck_words::table
        .filter(deck_words::deck_id.eq(payload.deck_id))
        .inner_join(words::table)
        .select((
            words::word_id,
            words::simplified,
            words::traditional,
            words::pinyin,
            words::definition,
        ))
        .load::<Word>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    Ok(Json(words))
}