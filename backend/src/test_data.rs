use sqlx::{query, SqlitePool};

/// Add some example data for testing during development.
pub async fn make_test_data(pool: &SqlitePool) -> anyhow::Result<()> {
    query!(
        r#"INSERT INTO auction_user (id, name, balance, login_key) VALUES
            (1, 'Alice', 100, 'aaa'),
            (2, 'Bob', 100, 'bbb'),
            (3, 'Carol', 100, 'ccc'),
            (4, 'Daniel', 100, 'ddd'),
            (5, 'Emily', 100, 'eee'),
            (6, 'Felix', 100, 'fff'),
            (7, 'Gabriel', 100, 'ggg'),
            (8, 'Hannah', 100, 'hhh'),
            (9, 'Ivan', 100, 'iii'),
            (10, 'Julia', 100, 'jjj'),
            (11, 'Kyle', 100, 'kkk'),
            (12, 'Lukas', 100, 'lll'),
            (13, 'Megan', 100, 'mmm'),
            (14, 'Natalie', 100, 'nnn'),
            (15, 'Oliver', 100, 'ooo'),
            (16, 'Penelope', 100, 'ppp'),
            (17, 'Quinn', 100, 'qqq'),
            (18, 'Ryan', 100, 'rrr'),
            (19, 'Sophia', 100, 'sss'),
            (20, 'Taylor', 100, 'ttt'),
            (21, 'Ulysses', 100, 'uuu'),
            (22, 'Victor', 100, 'vvv'),
            (23, 'William', 100, 'www'),
            (24, 'Xena', 100, 'xxx'),
            (25, 'Yosef', 100, 'yyy'),
            (26, 'Zachary', 100, 'zzz')
            "#
    )
    .execute(pool)
    .await?;
    query!("DELETE FROM auction_item WHERE 1=1")
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
    query!("DELETE FROM kv_data_int WHERE 1=1")
        .execute(pool)
        .await?;

    Ok(())
}
