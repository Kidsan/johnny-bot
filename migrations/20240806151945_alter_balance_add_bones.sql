-- Add migration script here
ALTER TABLE balances ADD COLUMN bones INTEGER DEFAULT 0;
