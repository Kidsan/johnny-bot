-- Add migration script here
DROP Table IF EXISTS role_holders; 
CREATE TABLE IF NOT EXISTS role_holders (
    role_id TEXT NOT NULL PRIMARY KEY,
    user_id TEXT NOT NULL
)
