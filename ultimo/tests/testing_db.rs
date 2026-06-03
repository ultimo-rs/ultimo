#![cfg(all(feature = "testing", feature = "sqlx-sqlite"))]

use ultimo::testing::with_test_transaction;

#[tokio::test]
async fn transaction_always_rolls_back() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .execute(&pool)
        .await
        .unwrap();

    with_test_transaction(&pool, |tx| {
        Box::pin(async move {
            sqlx::query("INSERT INTO t (id) VALUES (1)")
                .execute(&mut **tx)
                .await
                .unwrap();
            Ok(())
        })
    })
    .await
    .unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM t")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0, "rows must not persist after rollback");
}
