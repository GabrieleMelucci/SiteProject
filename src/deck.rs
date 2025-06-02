use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use diesel::sql_types::Integer;
use axum::extract::Path;

use crate::{schema::{deck_words, decks, words}, utils, DbPool};

#[derive(Serialize)]
pub struct DeckWord {
    pub id: i32,
    pub hanzi: Option<String>,
    pub pinyin: Option<String>,
    pub definition: Option<String>,
}

#[derive(Serialize)]
pub struct DeckWithWords {
    pub id: i32,
    pub name: String,
    pub words: Vec<DeckWord>,
}

#[derive(Deserialize)]
pub struct DeckId {
    pub deck_id: i32,
}

#[derive(Serialize)]
pub struct Deck {
    pub id: i32,
    pub name: String,
}

#[derive(Deserialize)]
pub struct CreateDeckRequest {
    pub name: String,
    pub word_id: Option<i32>,
    pub word_data: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct AddWordRequest {
    pub deck_id: i32,
    pub word_id: i32,
    pub word_data: serde_json::Value,
}

#[derive(Serialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, Queryable)]
pub struct Word {
    pub id: i32,
    pub word: String,
}

pub async fn list_decks(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
) -> Result<Json<Vec<Deck>>, (StatusCode, String)> {
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

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

pub async fn create_deck(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Json(payload): Json<CreateDeckRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Create the deck using the session user_id
    diesel::insert_into(decks::table)
        .values((
            decks::deck_name.eq(payload.name),
            decks::user_id.eq(user_id), 
        ))
        .execute(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    // Get the last inserted ID
    let deck_id = diesel::select(diesel::dsl::sql::<Integer>("last_insert_rowid()"))
        .get_result::<i32>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    // Add word to deck if provided
    if let (Some(word_id), Some(word_data)) = (payload.word_id, payload.word_data) {
        add_word_to_deck_internal(&mut conn, deck_id, word_id, word_data).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;
    }

    Ok(Json(ApiResponse {
        success: true,
        message: "Deck created successfully".to_string(),
    }))
}

pub async fn add_word_to_deck(
    State(pool): State<DbPool>,
    Json(payload): Json<AddWordRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    add_word_to_deck_internal(&mut conn, payload.deck_id, payload.word_id, payload.word_data)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Word added to deck successfully".to_string(),
    }))
}

fn add_word_to_deck_internal(
    conn: &mut SqliteConnection,
    deck_id: i32,
    word_id: i32,
    word_data: serde_json::Value,
) -> Result<(), diesel::result::Error> {
    // Insert or update word in words table
    diesel::insert_into(words::table)
        .values((
            words::word_id.eq(word_id),
            words::word.eq(word_data.to_string()),
        ))
        .on_conflict(words::word_id)
        .do_update()
        .set(words::word.eq(word_data.to_string()))
        .execute(conn)?;

    // Add relationship in deck_words table
    diesel::insert_into(deck_words::table)
        .values((
            deck_words::deck_id.eq(deck_id),
            deck_words::word_id.eq(word_id),
        ))
        .on_conflict((deck_words::deck_id, deck_words::word_id))
        .do_nothing()
        .execute(conn)?;

    Ok(())
}

pub async fn delete_deck(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Path(deck_id): Path<i32>,  // Change from Json to Path
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    let user_id = match utils::get_current_user_id(&session).await {
        Some(id) => id,
        None => return Err((StatusCode::UNAUTHORIZED, "Not logged in".to_string())),
    };

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    let deck_exists = decks::table
        .filter(decks::deck_id.eq(deck_id))  // Use the path parameter
        .filter(decks::user_id.eq(user_id))
        .count()
        .get_result::<i64>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })? > 0;

    if !deck_exists {
        return Err((StatusCode::NOT_FOUND, "Deck not found".to_string()));
    }

    // Transaction for atomic deletion
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        // First delete from deck_words
        diesel::delete(
            deck_words::table
                .filter(deck_words::deck_id.eq(deck_id))
        )
        .execute(conn)?;
        
        // Then delete from decks
        diesel::delete(
            decks::table
                .filter(decks::deck_id.eq(deck_id))
        )
        .execute(conn)
    }).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    Ok(Json(ApiResponse {
        success: true,
        message: "Deck deleted successfully".to_string(),
    }))
}

pub async fn view_deck(
    Path(deck_id): Path<i32>,
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
) -> Result<Json<DeckWithWords>, (StatusCode, String)> {
    let user_id = utils::get_current_user_id(&session).await.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "Not logged in".to_string())
    })?;

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Verify deck ownership and get deck name
    let (id, name): (i32, String) = decks::table
        .filter(decks::deck_id.eq(deck_id))
        .filter(decks::user_id.eq(user_id))
        .select((decks::deck_id, decks::deck_name))
        .first(&mut conn)
        .map_err(|_| {
            (StatusCode::NOT_FOUND, "Deck not found or access denied".to_string())
        })?;

    // Get words for the deck
    let words_data = deck_words::table
        .filter(deck_words::deck_id.eq(deck_id))
        .inner_join(words::table)
        .select((words::word_id, words::word))
        .load::<(i32, String)>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    // Parse each word's JSON data
    let mut words = Vec::new();
    for (id, word_json) in words_data {
        if let Ok(word_data) = serde_json::from_str::<serde_json::Value>(&word_json) {
            words.push(DeckWord {
                id,
                hanzi: word_data.get("simplified").and_then(|v| v.as_str()).map(|s| s.to_string()),
                pinyin: word_data.get("pinyin").and_then(|v| v.as_str()).map(|s| s.to_string()),
                definition: word_data.get("definitions")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<&str>>()
                        .join(", "))
            });
        }
    }

    Ok(Json(DeckWithWords {
        id,
        name,
        words,
    }))
}

pub async fn get_deck_words(
    State(pool): State<DbPool>,
    session: tower_sessions::Session,
    Json(payload): Json<DeckId>,
) -> Result<Json<Vec<Word>>, (StatusCode, String)> {
    let user_id = utils::get_current_user_id(&session).await.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "Not logged in".to_string())
    })?;

    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Verify ownership
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

    let words = deck_words::table
        .filter(deck_words::deck_id.eq(payload.deck_id))
        .inner_join(words::table)
        .select((words::word_id, words::word))
        .load::<Word>(&mut conn)
        .map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
        })?;

    Ok(Json(words))
}