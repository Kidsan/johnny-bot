-- Add migration script here
CREATE TABLE IF NOT EXISTS lottery_tickets (
    id INT NOT NULL PRIMARY KEY,
    tickets INT NOT NULL
);
