-- Add migration script here
CREATE TABLE IF NOT EXISTS dailies (
    id TEXT PRIMARY KEY,
    last_daily INTEGER NOT NULL
)

