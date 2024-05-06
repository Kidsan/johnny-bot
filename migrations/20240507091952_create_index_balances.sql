-- Add migration script here
CREATE INDEX IF NOT EXISTS idx_balances_balance ON balances (balance)
