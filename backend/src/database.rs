use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dotenvy::dotenv;
use std::env;

pub struct Database {
    pub conn: SqliteConnection,
}

impl Database {
    pub fn init() -> Self {
        dotenv().ok();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let conn = SqliteConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connectiong to {}", database_url));

        Self { conn }
    }
}
