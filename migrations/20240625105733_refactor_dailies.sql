-- Add migration script here
CREATE TABLE IF NOT EXISTS new_dailies (
    id BIGINT PRIMARY KEY,
    last_daily TIMESTAMP NOT NULL
);

INSERT into new_dailies (id, last_daily) SELECT CAST(id AS BIGINT) as id, datetime(last_daily, 'unixepoch') as last_daily FROM dailies;
DROP TABLE dailies;
ALTER TABLE new_dailies RENAME TO dailies;
