-- Add migration script here
CREATE TABLE IF NOT EXISTS kv_data_int (
    key TEXT NOT NULL PRIMARY KEY,
    value INTEGER NOT NULL
);