use sqlx::{SqlitePool, query};

/// Add some example data for testing during development.
pub async fn make_test_data(pool: &SqlitePool) -> anyhow::Result<()> {
    query!(r#"INSERT INTO auction_user (id, name, balance, login_key) VALUES
            (1, 'Alice', 1000, 'aaa'),
            (2, 'Bob', 500, 'bbb'),
            (3, 'Carol', 100, 'ccc')
            "#).execute(pool).await?;
    
    Ok(())
}