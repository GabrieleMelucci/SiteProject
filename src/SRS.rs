use chrono::{DateTime, Utc, NaiveDateTime};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = words)]
pub struct Word {
    pub word_id: i32,
    pub simplified: String,
    pub traditional: Option<String>,
    pub pinyin: String,
    pub definition: String,
    pub added_at: NaiveDateTime,
}

#[derive(Debug, Serialize, Deserialize, Insertable)]
#[diesel(table_name = srs_reviews)]
pub struct NewSrsReview {
    pub word_id: i32,
    pub deck_id: i32,
    pub user_id: i32,
    pub review_date: NaiveDateTime,
    pub next_review_date: NaiveDateTime,
    pub ease_factor: f64,
    pub interval: i32,
    pub performance: i32,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct SrsReview {
    pub review_id: i32,
    pub word_id: i32,
    pub deck_id: i32,
    pub user_id: i32,
    pub review_date: NaiveDateTime,
    pub next_review_date: NaiveDateTime,
    pub ease_factor: f64,
    pub interval: i32,
    pub performance: i32,
}

pub struct SrsEngine {
    db_conn: SqliteConnection,
}

impl SrsEngine {
    pub fn new(db_conn: SqliteConnection) -> Self {
        Self { db_conn }
    }

    pub fn record_review(
        &mut self,
        user_id: i32,
        deck_id: i32,
        word_id: i32,
        performance: i32,
    ) -> Result<SrsReview, diesel::result::Error> {
        use crate::schema::srs_reviews::dsl::*;
        
        // Get last review for this word-deck-user combination
        let last_review = srs_reviews
            .filter(word_id.eq(word_id))
            .filter(deck_id.eq(deck_id))
            .filter(user_id.eq(user_id))
            .order_by(review_date.desc())
            .first::<SrsReview>(&mut self.db_conn)
            .optional()?;

        // Calculate new SRS parameters (simplified SM-2 algorithm)
        let (new_ease, new_interval) = calculate_srs_parameters(performance, last_review);

        let now = Utc::now().naive_utc();
        let next_review = now + chrono::Duration::days(new_interval as i64);

        let new_review = NewSrsReview {
            word_id,
            deck_id,
            user_id,
            review_date: now,
            next_review_date: next_review,
            ease_factor: new_ease,
            interval: new_interval,
            performance,
        };

        diesel::insert_into(srs_reviews)
            .values(&new_review)
            .get_result(&mut self.db_conn)
    }

    pub fn get_due_words(
        &mut self,
        user_id: i32,
        deck_id: Option<i32>,
    ) -> Result<Vec<(Word, SrsReview)>, diesel::result::Error> {
        use crate::schema::{words, srs_reviews, deck_words};
        
        let mut query = srs_reviews::table
            .inner_join(words::table)
            .filter(srs_reviews::user_id.eq(user_id))
            .filter(srs_reviews::next_review_date.le(Utc::now().naive_utc()));

        if let Some(deck_id) = deck_id {
            query = query
                .inner_join(deck_words::table.on(
                    deck_words::word_id.eq(words::word_id).and(deck_words::deck_id.eq(deck_id))
                ))
                .filter(srs_reviews::deck_id.eq(deck_id));
        }

        query
            .select((words::all_columns, srs_reviews::all_columns))
            .load(&mut self.db_conn)
    }
}

// Helper function for SM-2 algorithm
fn calculate_srs_parameters(performance: i32, last_review: Option<SrsReview>) -> (f64, i32) {
    match last_review {
        Some(review) => {
            let mut ease = review.ease_factor;
            let mut interval = review.interval;

            match performance {
                0..=2 => { // Incorrect or hard
                    ease = (ease - 0.15).max(1.3);
                    interval = 1;
                },
                3 => { // Medium
                    interval = (interval as f64 * 1.5).ceil() as i32;
                },
                4..=5 => { // Correct or easy
                    ease += 0.1;
                    interval = (interval as f64 * ease).ceil() as i32;
                },
                _ => unreachable!(),
            }

            (ease, interval)
        }
        None => { // First review
            let ease = match performance {
                0..=2 => 2.0,
                3 => 2.3,
                4..=5 => 2.5,
                _ => unreachable!(),
            };
            (ease, 1)
        }
    }
}