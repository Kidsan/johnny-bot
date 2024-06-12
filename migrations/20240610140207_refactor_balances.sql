-- Add migration script here
CREATE TABLE IF NOT EXISTS new_balances (
  id BIGINT PRIMARY KEY,
  balance INTEGER NOT NULL
);

INSERT into new_balances (id, balance) SELECT CAST(id AS BIGINT) as id, balance FROM balances;
DROP TABLE balances;
ALTER TABLE new_balances RENAME TO balances;
