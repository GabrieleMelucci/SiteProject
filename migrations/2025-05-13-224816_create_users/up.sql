-- Your SQL goes here
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS userdecks;
DROP TABLE IF EXISTS deckword;

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
    definition TEXT NOT NULL      
);

create TABLE deck_words(
    deck_id INTEGER NOT NULL, 
    word_id INTEGER NOT NULL,
    
    PRIMARY KEY (word_id, deck_id),
    FOREIGN KEY (deck_id) REFERENCES decks(deck_id) ON DELETE CASCADE, 
    FOREIGN KEY (word_id) REFERENCES words(word_id) ON DELETE CASCADE
 )