use axum::{
    extract::{Json, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use diesel::prelude::*;
use diesel::sql_types::Integer;

use crate::{schema::{deck_words, decks, words}, utils, DbPool};

#[derive(Serialize)]
pub struct Deck {
    pub id: i32,
    pub name: String,
}

#[derive(Deserialize)]
pub struct CreateDeckRequest {
    pub name: String,
    pub user_id: i32,  // Added user_id to match schema
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
    Json(payload): Json<CreateDeckRequest>,
) -> Result<Json<ApiResponse>, (StatusCode, String)> {
    let mut conn = pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    // Create the deck with user_id
    diesel::insert_into(decks::table)
        .values((
            decks::deck_name.eq(payload.name),
            decks::user_id.eq(payload.user_id),
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