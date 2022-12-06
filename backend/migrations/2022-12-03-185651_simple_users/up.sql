CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  passcode TEXT NOT NULL,
  -- 0: Buyer, 1: Seller
  role INTEGER CHECK ( role IN (0, 1) ) NOT NULL
);
