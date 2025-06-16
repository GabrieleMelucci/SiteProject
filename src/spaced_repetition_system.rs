// spaced_repetition_system.rs
use chrono::{Duration, NaiveDateTime, Utc};
use diesel::prelude::*;

use crate::schema::srs_reviews;

/// Represents a review record in the SRS system
#[derive(Queryable, Insertable, Debug)]
#[diesel(table_name = srs_reviews)]
pub struct SrsReview {
    pub review_id: Option<i32>,
    pub word_id: i32,
    pub deck_id: i32,
    pub user_id: i32,
    pub review_date: NaiveDateTime,
    pub next_review_date: NaiveDateTime,
    pub ease_factor: f32,
    pub interval: i32,
    pub performance: i32,
}

/// The core SRS engine implementing SM-2 algorithm
pub struct SrsEngine<'a> {
    conn: &'a mut SqliteConnection,
}

impl<'a> SrsEngine<'a> {
    pub fn new(conn: &'a mut SqliteConnection) -> Self {
        SrsEngine { conn }
    }

    /// Records a review for a word and updates its SRS schedule
    pub fn record_review(
        &mut self,
        user_id: i32,
        deck_id: i32,
        word_id: i32,
        performance: i32,
    ) -> Result<(), diesel::result::Error> {
        // Get the last review if it exists
        let last_review = self.get_last_review(user_id, deck_id, word_id)?;

        // Calculate new SRS parameters based on performance
        let (interval, ease_factor) = match last_review {
            Some(review) => {
                self.calculate_srs_parameters(performance, review.interval, review.ease_factor)
            }
            None => self.initial_srs_parameters(performance),
        };

        // Calculate next review date
        let now = Utc::now().naive_utc();
        let next_review_date = now + Duration::days(interval as i64);

        // Use upsert (update or insert) operation
        diesel::insert_into(srs_reviews::table)
            .values((
                srs_reviews::word_id.eq(word_id),
                srs_reviews::deck_id.eq(deck_id),
                srs_reviews::user_id.eq(user_id),
                srs_reviews::review_date.eq(now),
                srs_reviews::next_review_date.eq(next_review_date),
                srs_reviews::ease_factor.eq(ease_factor),
                srs_reviews::interval.eq(interval),
                srs_reviews::performance.eq(performance),
            ))
            .on_conflict((srs_reviews::user_id, srs_reviews::word_id))
            .do_update()
            .set((
                srs_reviews::review_date.eq(now),
                srs_reviews::next_review_date.eq(next_review_date),
                srs_reviews::ease_factor.eq(ease_factor),
                srs_reviews::interval.eq(interval),
                srs_reviews::performance.eq(performance),
                srs_reviews::deck_id.eq(deck_id), 
            ))
            .execute(self.conn)?;

        Ok(())
    }

    /// Gets the last review for a word by a user in a deck
    pub fn get_last_review(
        &mut self,
        user_id: i32,
        deck_id: i32,
        word_id: i32,
    ) -> Result<Option<SrsReview>, diesel::result::Error> {
        srs_reviews::table
            .filter(srs_reviews::user_id.eq(user_id))
            .filter(srs_reviews::deck_id.eq(deck_id))
            .filter(srs_reviews::word_id.eq(word_id))
            .order_by(srs_reviews::review_date.desc())
            .first(self.conn)
            .optional()
    }

    /// Calculates initial SRS parameters based on first review performance
    fn initial_srs_parameters(&self, performance: i32) -> (i32, f32) {
        // Initial ease factor
        let ease_factor = 2.5;

        // Initial interval based on performance
        let interval = match performance {
            1 => 1, // Again - repeat next day
            2 => 1, // Hard - repeat next day
            3 => 3, // Good - repeat in 3 days
            4 => 5, // Easy - repeat in 5 days
            5 => 7, // Very Easy - repeat in 7 days
            _ => 1, // Default to 1 day for invalid values
        };

        (interval, ease_factor)
    }

    /// Updates SRS parameters based on performance and previous state
    fn calculate_srs_parameters(
        &self,
        performance: i32,
        previous_interval: i32,
        previous_ease: f32,
    ) -> (i32, f32) {
        // Calculate new ease factor (minimum 1.3)
        let mut ease_factor = previous_ease + (0.1 - (5 - performance) as f32 * 0.08);
        ease_factor = ease_factor.max(1.3);

        // Calculate new interval based on performance
        let interval = match performance {
            1 => 1, // Again - reset to 1 day
            2 => {
                // Hard - reset interval with 20% penalty
                (previous_interval as f32 * 0.8).max(1.0) as i32
            }
            3 | 4 | 5 => {
                // Good/Easy - multiply interval by ease factor
                (previous_interval as f32 * ease_factor).round() as i32
            }
            _ => previous_interval,
        };

        (interval, ease_factor)
    }
}
