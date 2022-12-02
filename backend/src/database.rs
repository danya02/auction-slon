use crate::data::{User, UserRole};
use rusqlite::Connection;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn init() -> Self {
        let conn = Connection::open_in_memory().expect("Could not open SQLite connection");
        conn.execute(
            "CREATE TABLE Users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                passcode TEXT NOT NULL,
                role TEXT NOT NULL
            )",
            (),
        )
        .expect("Unable to create table Users");

        // Note: this is mockup data
        let john_doe = User {
            id: 0,
            name: "John Doe".to_string(),
            passcode: "xXx_john-doe_xXx".to_string(),
            role: UserRole::Buyer,
        };

        let mary_sue = User {
            id: 1,
            name: "Mary Sue".to_string(),
            passcode: "mary-sue01".to_string(),
            role: UserRole::Buyer,
        };

        let harry_stew = User {
            id: 2,
            name: "Harry Stew".to_string(),
            passcode: "harry-stew".to_string(),
            role: UserRole::Seller,
        };

        for user in vec![john_doe, mary_sue, harry_stew] {
            conn.execute(
                "INSERT INTO Users (id, name, passcode, role) VALUES (?1, ?2, ?3, ?4)",
                (&user.id, &user.name, &user.passcode, &user.role),
            )
            .expect(&format!("Unable to insert user: {}", user.name));
        }

        Self { conn }
    }
}
