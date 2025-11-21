// Integration tests for database-sqlx example
//
// These tests demonstrate how to write integration tests for database operations.
// They require a running PostgreSQL instance.
//
// Run with: cargo test --example database-sqlx
// Or with database: DATABASE_URL=postgres://... cargo test --example database-sqlx

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

// Helper to get test database URL
fn get_test_db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/ultimo_test".to_string())
}

// Helper to create a test pool
async fn create_test_pool() -> sqlx::Pool<sqlx::Postgres> {
    let url = get_test_db_url();
    sqlx::PgPool::connect(&url)
        .await
        .expect("Failed to connect to test database. Make sure PostgreSQL is running.")
}

// Helper to setup test table
async fn setup_test_table(pool: &sqlx::Pool<sqlx::Postgres>) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS test_users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL UNIQUE
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create test table");
}

// Helper to cleanup test table
async fn cleanup_test_table(pool: &sqlx::Pool<sqlx::Postgres>) {
    sqlx::query("DROP TABLE IF EXISTS test_users")
        .execute(pool)
        .await
        .expect("Failed to drop test table");
}

#[tokio::test]
#[ignore] // Ignore by default - run with: cargo test -- --ignored
async fn test_create_user() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Test creating a user
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO test_users (name, email) VALUES ($1, $2) RETURNING id, name, email",
    )
    .bind("Alice")
    .bind("alice@example.com")
    .fetch_one(&pool)
    .await
    .expect("Failed to create user");

    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
    assert!(user.id > 0);

    cleanup_test_table(&pool).await;
}

#[tokio::test]
#[ignore]
async fn test_list_users() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Insert test data
    for (name, email) in [("Alice", "alice@example.com"), ("Bob", "bob@example.com")] {
        sqlx::query("INSERT INTO test_users (name, email) VALUES ($1, $2)")
            .bind(name)
            .bind(email)
            .execute(&pool)
            .await
            .expect("Failed to insert test user");
    }

    // Test listing users
    let users = sqlx::query_as::<_, User>("SELECT id, name, email FROM test_users ORDER BY id")
        .fetch_all(&pool)
        .await
        .expect("Failed to list users");

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");

    cleanup_test_table(&pool).await;
}

#[tokio::test]
#[ignore]
async fn test_get_user_by_id() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Insert test user
    let created_user = sqlx::query_as::<_, User>(
        "INSERT INTO test_users (name, email) VALUES ($1, $2) RETURNING id, name, email",
    )
    .bind("Alice")
    .bind("alice@example.com")
    .fetch_one(&pool)
    .await
    .expect("Failed to create user");

    // Test getting user by ID
    let fetched_user =
        sqlx::query_as::<_, User>("SELECT id, name, email FROM test_users WHERE id = $1")
            .bind(created_user.id)
            .fetch_one(&pool)
            .await
            .expect("Failed to fetch user");

    assert_eq!(fetched_user, created_user);

    cleanup_test_table(&pool).await;
}

#[tokio::test]
#[ignore]
async fn test_update_user() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Insert test user
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO test_users (name, email) VALUES ($1, $2) RETURNING id, name, email",
    )
    .bind("Alice")
    .bind("alice@example.com")
    .fetch_one(&pool)
    .await
    .expect("Failed to create user");

    // Test updating user
    let updated_user = sqlx::query_as::<_, User>(
        "UPDATE test_users SET name = $1 WHERE id = $2 RETURNING id, name, email",
    )
    .bind("Alice Smith")
    .bind(user.id)
    .fetch_one(&pool)
    .await
    .expect("Failed to update user");

    assert_eq!(updated_user.name, "Alice Smith");
    assert_eq!(updated_user.email, "alice@example.com"); // Email unchanged
    assert_eq!(updated_user.id, user.id);

    cleanup_test_table(&pool).await;
}

#[tokio::test]
#[ignore]
async fn test_delete_user() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Insert test user
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO test_users (name, email) VALUES ($1, $2) RETURNING id, name, email",
    )
    .bind("Alice")
    .bind("alice@example.com")
    .fetch_one(&pool)
    .await
    .expect("Failed to create user");

    // Test deleting user
    let result = sqlx::query("DELETE FROM test_users WHERE id = $1")
        .bind(user.id)
        .execute(&pool)
        .await
        .expect("Failed to delete user");

    assert_eq!(result.rows_affected(), 1);

    // Verify user is deleted
    let maybe_user =
        sqlx::query_as::<_, User>("SELECT id, name, email FROM test_users WHERE id = $1")
            .bind(user.id)
            .fetch_optional(&pool)
            .await
            .expect("Failed to query user");

    assert!(maybe_user.is_none());

    cleanup_test_table(&pool).await;
}

#[tokio::test]
#[ignore]
async fn test_transaction_rollback() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Start transaction
    let mut tx = pool.begin().await.expect("Failed to start transaction");

    // Insert user in transaction
    sqlx::query("INSERT INTO test_users (name, email) VALUES ($1, $2)")
        .bind("Alice")
        .bind("alice@example.com")
        .execute(&mut *tx)
        .await
        .expect("Failed to insert user");

    // Rollback transaction
    tx.rollback().await.expect("Failed to rollback");

    // Verify user was not persisted
    let users = sqlx::query_as::<_, User>("SELECT id, name, email FROM test_users")
        .fetch_all(&pool)
        .await
        .expect("Failed to query users");

    assert_eq!(users.len(), 0);

    cleanup_test_table(&pool).await;
}

#[tokio::test]
#[ignore]
async fn test_transaction_commit() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Start transaction
    let mut tx = pool.begin().await.expect("Failed to start transaction");

    // Insert multiple users in transaction
    for (name, email) in [("Alice", "alice@example.com"), ("Bob", "bob@example.com")] {
        sqlx::query("INSERT INTO test_users (name, email) VALUES ($1, $2)")
            .bind(name)
            .bind(email)
            .execute(&mut *tx)
            .await
            .expect("Failed to insert user");
    }

    // Commit transaction
    tx.commit().await.expect("Failed to commit");

    // Verify users were persisted
    let users = sqlx::query_as::<_, User>("SELECT id, name, email FROM test_users ORDER BY id")
        .fetch_all(&pool)
        .await
        .expect("Failed to query users");

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");

    cleanup_test_table(&pool).await;
}

#[tokio::test]
#[ignore]
async fn test_unique_constraint() {
    let pool = create_test_pool().await;
    setup_test_table(&pool).await;

    // Insert first user
    sqlx::query("INSERT INTO test_users (name, email) VALUES ($1, $2)")
        .bind("Alice")
        .bind("alice@example.com")
        .execute(&pool)
        .await
        .expect("Failed to insert user");

    // Try to insert user with duplicate email
    let result = sqlx::query("INSERT INTO test_users (name, email) VALUES ($1, $2)")
        .bind("Alice 2")
        .bind("alice@example.com") // Same email
        .execute(&pool)
        .await;

    // Should fail due to unique constraint
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("unique") || err_msg.contains("duplicate"));

    cleanup_test_table(&pool).await;
}

#[test]
fn test_user_serialization() {
    // Test that User struct can be serialized/deserialized
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let json = serde_json::to_string(&user).expect("Failed to serialize");
    assert!(json.contains("Alice"));
    assert!(json.contains("alice@example.com"));

    let deserialized: User = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized, user);
}

#[test]
fn test_create_user_input_deserialization() {
    let json = r#"{"name":"Alice","email":"alice@example.com"}"#;
    let input: CreateUserInput = serde_json::from_str(json).expect("Failed to deserialize");

    assert_eq!(input.name, "Alice");
    assert_eq!(input.email, "alice@example.com");
}
