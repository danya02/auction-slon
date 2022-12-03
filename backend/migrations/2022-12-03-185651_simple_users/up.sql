-- Your SQL goes here
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  passcode TEXT NOT NULL,
  -- 0: Buyer, 1: Seller
  role INTEGER CHECK ( role IN (0, 1) ) NOT NULL
);

-- Mockup user data
INSERT INTO users (name, passcode, role)
VALUES( "John Doe", "xXx_john-doe_xXx", 0);

INSERT INTO users (name, passcode, role)
VALUES( "Mary Sue", "mary-sue01", 0);

INSERT INTO users (name, passcode, role)
VALUES( "Harry Stew", "harry-stew", 1);
