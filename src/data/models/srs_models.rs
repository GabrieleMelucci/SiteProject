use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable};

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