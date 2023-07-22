-- Add migration script here
ALTER TABLE auction_user
ADD COLUMN sale_mode INTEGER NOT NULL DEFAULT 0;  -- 0 for normal buying, 1 for sharing money (UI mode)

ALTER TABLE auction_user
ADD COLUMN sponsorship_code TEXT DEFAULT NULL;  -- if NULL, sponsorship accepting disabled

CREATE UNIQUE INDEX auction_user_sponsorship_code ON auction_user(sponsorship_code);

CREATE TABLE IF NOT EXISTS sponsorship (
    id INTEGER PRIMARY KEY NOT NULL,
    donor_id INTEGER NOT NULL REFERENCES auction_user(id) ON DELETE CASCADE,
    recepient_id INTEGER NOT NULL REFERENCES auction_user(id) ON DELETE CASCADE,
    status INTEGER NOT NULL DEFAULT 0, 
    -- 0 for pending confirmation, 1 for accepted, 2 for rejected, 3 for retracted
    remaining_balance INTEGER NOT NULL DEFAULT 1
    -- while sponsorship is active, this can be adjusted up and down
);

CREATE TABLE IF NOT EXISTS sale_contribution (
    sale_id INTEGER NOT NULL REFERENCES auction_item_sale(item_id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES auction_user(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL
);