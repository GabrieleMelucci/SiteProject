-- Your SQL goes here
DROP TABLE IF EXISTS users;

CREATE TABLE users (
    id INTEGER PRIMARY KEY NOT NULL,  -- Changed to NOT NULL
    username TEXT NOT NULL,
    password TEXT NOT NULL
);