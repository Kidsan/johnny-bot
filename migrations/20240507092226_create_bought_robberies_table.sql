-- Add migration script here
CREATE TABLE IF NOT EXISTS bought_robberies (
    id TEXT PRIMARY KEY,
    last_bought INTEGER NOT NULL
)
