-- Add migration script here
CREATE TABLE new_role_holders (
    role_id BIGINT PRIMARY KEY,
    user_id BIGINT NOT NULL,
    purchased TIMESTAMP
);

INSERT into new_role_holders (role_id, user_id, purchased) SELECT CAST(role_id AS BIGINT) as role_id, CAST(user_id as BIGINT) as user_id, purchased FROM role_holders;
DROP TABLE role_holders;
ALTER TABLE new_role_holders RENAME TO role_holders;
