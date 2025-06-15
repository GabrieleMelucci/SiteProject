// @generated automatically by Diesel CLI.

diesel::table! {
    deck_words (deck_id, word_id) {
        deck_id -> Integer,
        word_id -> Integer,
    }
}

diesel::table! {
    decks (deck_id) {
        deck_id -> Integer,
        user_id -> Integer,
        deck_name -> Text,
    }
}

diesel::table! {
    srs_reviews (review_id) {
        review_id -> Nullable<Integer>,
        word_id -> Integer,
        deck_id -> Integer,
        user_id -> Integer,
        review_date -> Timestamp,
        next_review_date -> Timestamp,
        ease_factor -> Float,
        interval -> Integer,
        performance -> Integer,
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Integer,
        email -> Text,
        password -> Text,
    }
}

diesel::table! {
    words (word_id) {
        word_id -> Integer,
        simplified -> Text,
        traditional -> Nullable<Text>,
        pinyin -> Text,
        definition -> Text,
        added_at -> Timestamp,
    }
}

diesel::joinable!(deck_words -> decks (deck_id));
diesel::joinable!(deck_words -> words (word_id));
diesel::joinable!(decks -> users (user_id));
diesel::joinable!(srs_reviews -> decks (deck_id));
diesel::joinable!(srs_reviews -> users (user_id));
diesel::joinable!(srs_reviews -> words (word_id));

diesel::allow_tables_to_appear_in_same_query!(
    deck_words,
    decks,
    srs_reviews,
    users,
    words,
);
