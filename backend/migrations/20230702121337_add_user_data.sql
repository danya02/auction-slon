-- Add migration script here
CREATE TABLE IF NOT EXISTS auction_user (
    id INTEGER NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    balance INTEGER NOT NULL DEFAULT 0,
    login_key TEXT NOT NULL
);