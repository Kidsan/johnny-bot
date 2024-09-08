-- Add migration script here
CREATE TABLE IF NOT EXISTS reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    description TEXT NOT NULL,
    link TEXT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
