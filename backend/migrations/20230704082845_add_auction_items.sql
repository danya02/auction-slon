-- Add migration script here
CREATE TABLE auction_item (
    id INTEGER PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    initial_price INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE auction_item_sale (
    item_id INTEGER UNIQUE NOT NULL,
    buyer_id INTEGER NOT NULL,
    sale_price INTEGER NOT NULL,
    FOREIGN KEY (item_id) REFERENCES auction_item(id),
    FOREIGN KEY (buyer_id) REFERENCES auction_user(id)
);
