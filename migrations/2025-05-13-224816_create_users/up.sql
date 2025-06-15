DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS decks;
DROP TABLE IF EXISTS words;
DROP TABLE IF EXISTS deck_words;
DROP TABLE IF EXISTS srs_reviews;

CREATE TABLE users (
    user_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,  
    email TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);

CREATE TABLE decks (
    deck_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,  
    user_id INTEGER NOT NULL,
    deck_name TEXT NOT NULL,
    
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE TABLE words (
    word_id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    simplified TEXT NOT NULL,        
    traditional TEXT,               
    pinyin TEXT NOT NULL,            
    definition TEXT NOT NULL, 
    added_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP    
);

CREATE TABLE deck_words (
    deck_id INTEGER NOT NULL, 
    word_id INTEGER NOT NULL,
    
    PRIMARY KEY (deck_id, word_id),
    FOREIGN KEY (deck_id) REFERENCES decks(deck_id) ON DELETE CASCADE, 
    FOREIGN KEY (word_id) REFERENCES words(word_id) ON DELETE CASCADE
);

CREATE TABLE srs_reviews (
    review_id INTEGER PRIMARY KEY AUTOINCREMENT,
    word_id INTEGER NOT NULL,
    deck_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    review_date TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    next_review_date TIMESTAMP NOT NULL,
    ease_factor REAL NOT NULL DEFAULT 2.5,
    interval INTEGER NOT NULL DEFAULT 1,
    performance INTEGER NOT NULL,
    
    FOREIGN KEY (word_id) REFERENCES words(word_id) ON DELETE CASCADE,
    FOREIGN KEY (deck_id) REFERENCES decks(deck_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_srs_reviews_user ON srs_reviews(user_id);
CREATE INDEX idx_srs_reviews_due ON srs_reviews(next_review_date);