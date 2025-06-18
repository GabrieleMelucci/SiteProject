use axum::extract::Path;
use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use diesel::prelude::*;
use diesel::sql_types::Integer;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDateTime, Utc};

use crate::{
    DbPool,
    schema::{deck_words, decks, words, srs_reviews},
    utils,
    spaced_repetition_system::SrsEngine
};

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
        .select((decks::deck_id, decks::deck_name, decks::privacy_value))
        .load::<(i32, String, bool)>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?
        .into_iter()
        .map(|(id, name, privacy_value)| Deck { id, name, privacy_value })
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
                decks::privacy_value.eq(payload.privacy_value),
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
    match add_word_to_deck_internal(&mut conn, payload.deck_id, payload.word_data) {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            message: "Word added to deck successfully".to_string(),
        })),
        Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            _,
        )) => Err((StatusCode::CONFLICT, "Word already exists in this deck".to_string())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )),
    }
}

/// Internal function to handle word addition to a deck
/// Returns the auto-generated word ID
fn add_word_to_deck_internal(
    conn: &mut SqliteConnection,
    deck_id: i32,
    word_data: serde_json::Value,
) -> Result<i32, diesel::result::Error> {
    // Extract word data with proper error handling
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

    // First try to find existing word
    let existing_word = words::table
        .filter(words::simplified.eq(simplified))
        .filter(words::pinyin.eq(pinyin))
        .filter(words::definition.eq(&definition))
        .select(words::word_id)
        .first::<i32>(conn)
        .optional()?;

    let word_id = match existing_word {
        Some(id) => id, // Use existing word
        None => {
            // Insert new word
            diesel::insert_into(words::table)
                .values((
                    words::simplified.eq(simplified),
                    words::traditional.eq(traditional),
                    words::pinyin.eq(pinyin),
                    words::definition.eq(definition),
                ))
                .execute(conn)?;
            
            diesel::select(diesel::dsl::sql::<Integer>("last_insert_rowid()"))
                .get_result::<i32>(conn)?
        }
    };

    // Check if word already exists in deck
    let word_in_deck: i64 = deck_words::table
        .filter(deck_words::deck_id.eq(deck_id))
        .filter(deck_words::word_id.eq(word_id))
        .count()
        .get_result(conn)?;

    if word_in_deck > 0 {
        return Err(diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation,
            Box::new("Word already exists in this deck".to_string())
        ));
    }

    // Add to deck_words
    diesel::insert_into(deck_words::table)
        .values((
            deck_words::deck_id.eq(deck_id),
            deck_words::word_id.eq(word_id),
        ))
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

//Deletes a word from a deck
pub async fn delete_word_from_deck(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Path((deck_id, word_id)): Path<(i32, i32)>,
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

    // Delete only the word-deck connection
    diesel::delete(
        deck_words::table
            .filter(deck_words::deck_id.eq(deck_id))
            .filter(deck_words::word_id.eq(word_id)),
    )
    .execute(&mut conn)
    .map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Word removed from deck successfully".to_string(),
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
            deck_id,
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

pub async fn start_study_session(
    Path(deck_id): Path<i32>,
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
) -> Result<Json<Vec<StudyWord>>, (StatusCode, String)> {
    // Get current user ID
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Verify deck ownership
    let deck_owner: i32 = decks::table
        .filter(decks::deck_id.eq(deck_id))
        .select(decks::user_id)
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Deck not found".to_string()))?;

    if deck_owner != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    // Get all words in the deck
    let words_with_added_at = deck_words::table
        .filter(deck_words::deck_id.eq(deck_id))
        .inner_join(words::table)
        .select((
            words::word_id,
            words::simplified,
            words::traditional,
            words::pinyin,
            words::definition,
            words::added_at,
        ))
        .load::<(i32, String, Option<String>, String, String, NaiveDateTime)>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to load words: {}", e))
        })?;

    // Create SRS engine instance
    let mut srs_engine = SrsEngine::new(&mut conn);

    // Process each word - treat missing SRS data as new words
    let mut study_words = Vec::new();
    for (word_id, simplified, traditional, pinyin, definition, _added_at) in words_with_added_at {
        let last_review = srs_engine.get_last_review(user_id, deck_id, word_id).ok();
        
        study_words.push(StudyWord {
            word: DeckWord {
                id: word_id,
                simplified,
                traditional,
                pinyin,
                definition,
                deck_id,
            },
            is_new: last_review.is_none(),
            last_performance: last_review.as_ref().map(|r| r.as_ref().map(|rev| rev.performance)).flatten(),
            next_review: last_review.as_ref().and_then(|r| r.as_ref().map(|rev| rev.next_review_date)),
        });
    }

    // Sort words: new words first, then by next review date (earliest first)
    study_words.sort_by(|a, b| {
        match (a.is_new, b.is_new) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => match (a.next_review, b.next_review) {
                (Some(a_date), Some(b_date)) => a_date.cmp(&b_date),
                _ => std::cmp::Ordering::Equal,
            },
        }
    });

    Ok(Json(study_words))
}

#[axum::debug_handler]
pub async fn record_word_review(
    Path((deck_id, word_id)): Path<(i32, i32)>,
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Json(payload): Json<ReviewRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    // Validate performance rating
    if payload.performance < 1 || payload.performance > 5 {
        return Err((StatusCode::BAD_REQUEST, "Performance must be between 1 and 5".to_string()));
    }

    // Verify user is logged in
    let user_id = utils::get_current_user_id(&session)
        .await
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Not logged in".to_string()))?;

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Verify deck ownership and that the word is in the deck
    let deck_owner: i32 = decks::table
        .filter(decks::deck_id.eq(deck_id))
        .select(decks::user_id)
        .first(&mut conn)
        .map_err(|_| {
            (StatusCode::FORBIDDEN, "Deck not found or access denied".to_string())
        })?;

    if deck_owner != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    // Verify word exists in deck
    let word_in_deck: i64 = deck_words::table
        .filter(deck_words::deck_id.eq(deck_id))
        .filter(deck_words::word_id.eq(word_id))
        .count()
        .get_result(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    if word_in_deck == 0 {
        return Err((StatusCode::NOT_FOUND, "Word not found in deck".to_string()));
    }

    // Record the review using SRS engine
    let mut srs_engine = SrsEngine::new(&mut conn);

    srs_engine.record_review(user_id, deck_id, word_id, payload.performance)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Review recorded successfully".to_string(),
    }))
}

pub async fn get_all_due_words(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
) -> Result<Json<Vec<StudyWord>>, (StatusCode, String)> {
    // Get current user ID
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Get all due words with their details
    let study_words = srs_reviews::table
        .filter(srs_reviews::user_id.eq(user_id))
        .filter(srs_reviews::next_review_date.le(Utc::now().naive_utc()))
        .inner_join(words::table.on(srs_reviews::word_id.eq(words::word_id)))
        .select((
            words::word_id,
            words::simplified,
            words::traditional,
            words::pinyin,
            words::definition,
            srs_reviews::deck_id,
            srs_reviews::performance,
            srs_reviews::next_review_date,
        ))
        .load::<(i32, String, Option<String>, String, String, i32, i32, NaiveDateTime)>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?
        .into_iter()
        .map(|(word_id, simplified, traditional, pinyin, definition, deck_id, performance, next_review_date)| {
            StudyWord {
                word: DeckWord {
                    id: word_id,
                    simplified,
                    traditional,
                    pinyin,
                    definition,
                    deck_id,
                },
                is_new: false, 
                last_performance: Some(performance),
                next_review: Some(next_review_date),
            }
        })
        .collect::<Vec<_>>();

    Ok(Json(study_words))
}

#[axum::debug_handler]
pub async fn get_due_words_count(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
) -> Result<Json<i32>, (StatusCode, String)> {
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    let count = srs_reviews::table
        .filter(srs_reviews::user_id.eq(user_id))
        .filter(srs_reviews::next_review_date.le(Utc::now().naive_utc()))
        .count()
        .get_result::<i64>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    Ok(Json(count as i32))
}

pub async fn update_deck_privacy(
    Path(deck_id): Path<i32>,
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Json(payload): Json<UpdatePrivacyRequest>,
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
    let deck_owner: i32 = decks::table
        .filter(decks::deck_id.eq(deck_id))
        .select(decks::user_id)
        .first(&mut conn)
        .map_err(|_| {
            (StatusCode::FORBIDDEN, "Deck not found or access denied".to_string())
        })?;

    if deck_owner != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    // Update the privacy setting
    diesel::update(decks::table.filter(decks::deck_id.eq(deck_id)))
        .set(decks::privacy_value.eq(payload.privacy_value))
        .execute(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Privacy setting updated successfully".to_string(),
    }))
}