-- Add migration script here
CREATE TABLE IF NOT EXISTS new_purchaseable_roles (
    role_id BIGINT PRIMARY KEY,
    price INTEGER NOT NULL,
    increment INT DEFAULT 0,
    required_role_id BIGINT DEFAULT NULL, 
    only_one boolean default false
);


INSERT into new_purchaseable_roles (role_id, price, increment, required_role_id, only_one) 
SELECT CAST(role_id AS BIGINT) as role_id, price, increment, CAST(required_role_id AS BIGINT) as required_role_id, only_one from purchaseable_roles;
DROP TABLE purchaseable_roles;
ALTER TABLE new_purchaseable_roles RENAME TO purchaseable_roles;
