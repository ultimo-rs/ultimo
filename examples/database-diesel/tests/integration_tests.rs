// Integration tests for database-diesel example
//
// These tests demonstrate how to write integration tests with Diesel ORM.
// They require a running PostgreSQL instance.
//
// Run with: cargo test --example database-diesel
// Or with database: DATABASE_URL=postgres://... cargo test --example database-diesel

use diesel::prelude::*;
use diesel::result::Error as DieselError;
use serde::{Deserialize, Serialize};

// Schema definition
diesel::table! {
    test_users (id) {
        id -> Int4,
        name -> Varchar,
        email -> Varchar,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, PartialEq)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = test_users)]
struct NewUser<'a> {
    name: &'a str,
    email: &'a str,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = test_users)]
struct UpdateUser<'a> {
    name: Option<&'a str>,
    email: Option<&'a str>,
}

// Helper to get test database URL
fn get_test_db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/ultimo_test".to_string())
}

// Helper to create test connection
fn create_test_connection() -> PgConnection {
    let url = get_test_db_url();
    PgConnection::establish(&url)
        .expect("Failed to connect to test database. Make sure PostgreSQL is running.")
}

// Helper to setup test table
fn setup_test_table(conn: &mut PgConnection) {
    diesel::sql_query(
        "CREATE TABLE IF NOT EXISTS test_users (
            id SERIAL PRIMARY KEY,
            name VARCHAR NOT NULL,
            email VARCHAR NOT NULL UNIQUE
        )",
    )
    .execute(conn)
    .expect("Failed to create test table");
}

// Helper to cleanup test table
fn cleanup_test_table(conn: &mut PgConnection) {
    diesel::sql_query("DROP TABLE IF EXISTS test_users")
        .execute(conn)
        .expect("Failed to drop test table");
}

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_create_user() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    let new_user = NewUser {
        name: "Alice",
        email: "alice@example.com",
    };

    let user: User = diesel::insert_into(test_users::table)
        .values(&new_user)
        .get_result(&mut conn)
        .expect("Failed to insert user");

    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
    assert!(user.id > 0);

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_list_users() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    // Insert test data
    let users_data = vec![
        NewUser {
            name: "Alice",
            email: "alice@example.com",
        },
        NewUser {
            name: "Bob",
            email: "bob@example.com",
        },
    ];

    for user in users_data {
        diesel::insert_into(test_users::table)
            .values(&user)
            .execute(&mut conn)
            .expect("Failed to insert user");
    }

    // Query all users
    let users = test_users::table
        .order(test_users::id)
        .load::<User>(&mut conn)
        .expect("Failed to load users");

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_get_user_by_id() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    // Insert test user
    let new_user = NewUser {
        name: "Alice",
        email: "alice@example.com",
    };

    let created_user: User = diesel::insert_into(test_users::table)
        .values(&new_user)
        .get_result(&mut conn)
        .expect("Failed to insert user");

    // Query user by ID
    let fetched_user: User = test_users::table
        .find(created_user.id)
        .first(&mut conn)
        .expect("Failed to find user");

    assert_eq!(fetched_user, created_user);

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_update_user() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    // Insert test user
    let new_user = NewUser {
        name: "Alice",
        email: "alice@example.com",
    };

    let user: User = diesel::insert_into(test_users::table)
        .values(&new_user)
        .get_result(&mut conn)
        .expect("Failed to insert user");

    // Update user
    let update = UpdateUser {
        name: Some("Alice Smith"),
        email: None,
    };

    let updated_user: User = diesel::update(test_users::table.find(user.id))
        .set(&update)
        .get_result(&mut conn)
        .expect("Failed to update user");

    assert_eq!(updated_user.name, "Alice Smith");
    assert_eq!(updated_user.email, "alice@example.com"); // Unchanged
    assert_eq!(updated_user.id, user.id);

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_delete_user() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    // Insert test user
    let new_user = NewUser {
        name: "Alice",
        email: "alice@example.com",
    };

    let user: User = diesel::insert_into(test_users::table)
        .values(&new_user)
        .get_result(&mut conn)
        .expect("Failed to insert user");

    // Delete user
    let deleted_count = diesel::delete(test_users::table.find(user.id))
        .execute(&mut conn)
        .expect("Failed to delete user");

    assert_eq!(deleted_count, 1);

    // Verify user is deleted
    let result = test_users::table.find(user.id).first::<User>(&mut conn);

    assert!(matches!(result, Err(DieselError::NotFound)));

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_transaction_rollback() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    let result: Result<(), DieselError> = conn.transaction(|conn| {
        // Insert user
        let new_user = NewUser {
            name: "Alice",
            email: "alice@example.com",
        };

        diesel::insert_into(test_users::table)
            .values(&new_user)
            .execute(conn)?;

        // Force rollback by returning an error
        Err(DieselError::RollbackTransaction)
    });

    assert!(matches!(result, Err(DieselError::RollbackTransaction)));

    // Verify user was not persisted
    let users = test_users::table
        .load::<User>(&mut conn)
        .expect("Failed to load users");

    assert_eq!(users.len(), 0);

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_transaction_commit() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    conn.transaction::<_, DieselError, _>(|conn| {
        // Insert multiple users
        let users = vec![
            NewUser {
                name: "Alice",
                email: "alice@example.com",
            },
            NewUser {
                name: "Bob",
                email: "bob@example.com",
            },
        ];

        for user in users {
            diesel::insert_into(test_users::table)
                .values(&user)
                .execute(conn)?;
        }

        Ok(())
    })
    .expect("Transaction failed");

    // Verify users were persisted
    let users = test_users::table
        .order(test_users::id)
        .load::<User>(&mut conn)
        .expect("Failed to load users");

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[1].name, "Bob");

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_unique_constraint() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    // Insert first user
    let user1 = NewUser {
        name: "Alice",
        email: "alice@example.com",
    };

    diesel::insert_into(test_users::table)
        .values(&user1)
        .execute(&mut conn)
        .expect("Failed to insert first user");

    // Try to insert user with duplicate email
    let user2 = NewUser {
        name: "Alice 2",
        email: "alice@example.com", // Same email
    };

    let result = diesel::insert_into(test_users::table)
        .values(&user2)
        .execute(&mut conn);

    // Should fail due to unique constraint
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, DieselError::DatabaseError(_, _)));

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_filter_and_count() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    // Insert test data
    let users = vec![
        NewUser {
            name: "Alice",
            email: "alice@example.com",
        },
        NewUser {
            name: "Bob",
            email: "bob@example.com",
        },
        NewUser {
            name: "Alice Smith",
            email: "alice.smith@example.com",
        },
    ];

    for user in users {
        diesel::insert_into(test_users::table)
            .values(&user)
            .execute(&mut conn)
            .expect("Failed to insert user");
    }

    // Filter users whose name starts with "Alice"
    let alice_users = test_users::table
        .filter(test_users::name.like("Alice%"))
        .load::<User>(&mut conn)
        .expect("Failed to filter users");

    assert_eq!(alice_users.len(), 2);

    // Count all users
    let total_count: i64 = test_users::table
        .count()
        .get_result(&mut conn)
        .expect("Failed to count users");

    assert_eq!(total_count, 3);

    cleanup_test_table(&mut conn);
}

#[test]
#[ignore]
fn test_batch_insert() {
    let mut conn = create_test_connection();
    setup_test_table(&mut conn);

    // Batch insert multiple users
    let new_users = vec![
        NewUser {
            name: "Alice",
            email: "alice@example.com",
        },
        NewUser {
            name: "Bob",
            email: "bob@example.com",
        },
        NewUser {
            name: "Carol",
            email: "carol@example.com",
        },
    ];

    let inserted_count = diesel::insert_into(test_users::table)
        .values(&new_users)
        .execute(&mut conn)
        .expect("Failed to batch insert");

    assert_eq!(inserted_count, 3);

    // Verify all users were inserted
    let users = test_users::table
        .load::<User>(&mut conn)
        .expect("Failed to load users");

    assert_eq!(users.len(), 3);

    cleanup_test_table(&mut conn);
}

#[test]
fn test_user_serialization() {
    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let json = serde_json::to_string(&user).expect("Failed to serialize");
    assert!(json.contains("Alice"));

    let deserialized: User = serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized, user);
}

#[test]
fn test_new_user_struct() {
    let new_user = NewUser {
        name: "Alice",
        email: "alice@example.com",
    };

    assert_eq!(new_user.name, "Alice");
    assert_eq!(new_user.email, "alice@example.com");
}
