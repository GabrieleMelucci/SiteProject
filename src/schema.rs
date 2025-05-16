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
    }
}

diesel::table! {
    users (user_id) {
        user_id -> Integer,
        email -> Text,
        username -> Text,
        password -> Text,
    }
}

diesel::table! {
    words (word_id) {
        word_id -> Integer,
        word -> Text,
    }
}

diesel::joinable!(deck_words -> decks (deck_id));
diesel::joinable!(deck_words -> words (word_id));
diesel::joinable!(decks -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    deck_words,
    decks,
    users,
    words,
);
