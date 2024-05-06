-- Add migration script here
CREATE TABLE IF NOT EXISTS balances (
    id TEXT PRIMARY KEY,
    balance INTEGER NOT NULL
)
