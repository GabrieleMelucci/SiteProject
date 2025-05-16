-- Your SQL goes here
DROP TABLE IF EXISTS users;

CREATE TABLE users (
    id INTEGER PRIMARY KEY NOT NULL,  
    username TEXT NOT NULL UNIQUE,
    password TEXT NOT NULL
);