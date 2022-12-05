use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use std::path::PathBuf;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() {
    let mut dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dotenv_path.push("src/bin/testing/.env");
    dotenvy::from_path(dotenv_path.clone()).expect(&format!(
        "Could not find .env file at {}",
        dotenv_path.to_string_lossy()
    ));
    pretty_env_logger::init();

    {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        println!("{}", database_url);
        let mut conn = SqliteConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connectiong to {}", database_url));
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Could not run pending migrations");
        // Todo: use rusqlite to run raw SQL
    }

    backend::run([127, 0, 0, 1], 3030).await;
}
