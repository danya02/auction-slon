use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let mut dotenv_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dotenv_path.push("src/bin/dev/.env");
    dotenvy::from_path(dotenv_path.clone()).expect(&format!(
        "Could not find .env file at {}",
        dotenv_path.to_string_lossy()
    ));
    pretty_env_logger::init();
    init_mock_db();
    backend::run([127, 0, 0, 1], 3030).await;
}

fn init_mock_db() {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let mut conn = SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    use backend::schema::users::dsl::*;

    // Delete all previous data
    // We do not care if no tables existed prior, so ignore the err
    diesel::delete(users).execute(&mut conn).ok();

    // Run all migrations
    const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Could not run pending migrations");

    // Add mock data
    for (n, p, r) in mock_data() {
        diesel::insert_into(users)
            .values((name.eq(&n), passcode.eq(&p), role.eq(&r)))
            .execute(&mut conn)
            .expect(&format!(
                "Could not insert ({}, {}, {}) into `users`",
                n, p, r
            ));
    }
}

fn mock_data() -> Vec<(String, String, i32)> {
    vec![
        ("John Doe".to_string(), "xXx_john-doe_xXx".to_string(), 0),
        ("Mary Sue".to_string(), "mary-sue01".to_string(), 0),
        ("Harry Stew".to_string(), "harry-stew".to_string(), 1),
    ]
}
