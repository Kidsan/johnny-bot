-- Add migration script here
CREATE TABLE IF NOT EXISTS paid_channels (
    id INTEGER PRIMARY KEY,
    price INTEGER NOT NULL
)
