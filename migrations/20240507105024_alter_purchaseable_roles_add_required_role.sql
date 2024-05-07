-- Add migration script here
ALTER TABLE purchaseable_roles ADD COLUMN required_role_id TEXT DEFAULT NULL;
