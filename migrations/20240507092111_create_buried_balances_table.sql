-- Add migration script here
CREATE TABLE IF NOT EXISTS buried_balances (
    id TEXT PRIMARY KEY,
    amount INTEGER NOT NULL
)
