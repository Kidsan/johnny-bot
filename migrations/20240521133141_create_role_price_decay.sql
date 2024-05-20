-- Add migration script here
CREATE TABLE IF NOT EXISTS role_price_decay (
    role_id INT NOT NULL PRIMARY KEY,
    amount INT NOT NULL,
    last_decay TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    interval INT NOT NULL,
    minimum INT NOT NULL
)
