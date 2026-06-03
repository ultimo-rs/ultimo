//! Database test helpers — always roll back so tests never mutate state.

use crate::error::{Result, UltimoError};
use std::future::Future;
use std::pin::Pin;

/// Run `f` inside a transaction that is **always rolled back**, regardless of
/// the closure's result. Lets a test exercise real queries against a real
/// database without persisting any changes.
///
/// The closure receives `&mut Transaction` and returns a boxed future — mirror
/// sqlx's own [`Pool::transaction`] shape: `|tx| Box::pin(async move { … })`.
///
/// Available when the `sqlx` feature (via any `sqlx-*` backend) is enabled.
///
/// ```no_run
/// # #[cfg(feature = "sqlx-sqlite")]
/// # async fn demo() -> ultimo::Result<()> {
/// use ultimo::testing::with_test_transaction;
/// let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
/// with_test_transaction(&pool, |tx| {
///     Box::pin(async move {
///         sqlx::query("INSERT INTO t (id) VALUES (1)")
///             .execute(&mut **tx)
///             .await
///             .unwrap();
///         Ok(())
///     })
/// })
/// .await?; // the insert is rolled back here
/// # Ok(())
/// # }
/// ```
#[cfg(feature = "sqlx")]
pub async fn with_test_transaction<DB, F, T>(pool: &sqlx::Pool<DB>, f: F) -> Result<T>
where
    DB: sqlx::Database,
    for<'c> F: FnOnce(
        &'c mut sqlx::Transaction<'_, DB>,
    ) -> Pin<Box<dyn Future<Output = Result<T>> + Send + 'c>>,
{
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| UltimoError::Internal(format!("begin test transaction: {e}")))?;
    let result = f(&mut tx).await;
    // Always roll back — never commit — so tests leave no trace.
    let _ = tx.rollback().await;
    result
}
