use sqlx::{query, SqlitePool};

/// Add some example data for testing during development.
pub async fn make_test_data(pool: &SqlitePool) -> anyhow::Result<()> {
    query!(
        r#"INSERT INTO auction_user (id, name, balance, login_key) VALUES
            (1, 'Alice', 1000, 'aaa'),
            (2, 'Bob', 500, 'bbb'),
            (3, 'Carol', 100, 'ccc')
            "#
    )
    .execute(pool)
    .await?;

    query!(
        r#"INSERT INTO auction_item (id, name, initial_price) VALUES
            (1, 'Widget', 50),
            (2, 'Gadget', 100),
            (3, 'Frobjet', 500)
            "#
    )
    .execute(pool)
    .await?;

    Ok(())
}
