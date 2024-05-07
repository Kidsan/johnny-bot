-- Add migration script here
CREATE TABLE IF NOT EXISTS role_holders (
    role_id TEXT NOT NULL,
    user_id TEXT NOT NULL
)
