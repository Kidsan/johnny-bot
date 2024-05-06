-- Add migration script here
CREATE TABLE IF NOT EXISTS purchaseable_roles (
    role_id TEXT PRIMARY KEY,
    price INTEGER NOT NULL,
    only_one BOOLEAN NOT NULL DEFAULT FALSE
)
