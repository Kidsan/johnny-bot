-- Add migration script here
CREATE TABLE community_emojis (
    name TEXT NOT NULL PRIMARY KEY,
    added TIMESTAMP NOT NULL
);
