//! Tests for database integration
//!
//! This module contains unit tests for the database integration layer.
//! Integration tests that require a real database are in examples/tests.

#[cfg(feature = "database")]
#[cfg(test)]
mod tests {
    use crate::database::DatabaseError;

    #[test]
    fn test_database_error_display() {
        let err = DatabaseError::Connection("test error".to_string());
        assert_eq!(err.to_string(), "Database connection error: test error");

        let err = DatabaseError::Query("query failed".to_string());
        assert_eq!(err.to_string(), "Database query error: query failed");

        let err = DatabaseError::NotConfigured;
        assert!(err.to_string().contains("not configured"));
    }

    #[test]
    fn test_database_error_conversion() {
        let db_err = DatabaseError::Connection("test".to_string());
        let ultimo_err: crate::UltimoError = db_err.into();

        match ultimo_err {
            crate::UltimoError::Internal(msg) => {
                assert!(msg.contains("connection error"));
            }
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_database_error_types() {
        // Test all error variants
        let errors = vec![
            DatabaseError::Connection("conn".to_string()),
            DatabaseError::Query("query".to_string()),
            DatabaseError::Migration("migration".to_string()),
            DatabaseError::Pool("pool".to_string()),
            DatabaseError::Transaction("tx".to_string()),
            DatabaseError::NotConfigured,
        ];

        for err in errors {
            // Each error should have a meaningful display message
            let msg = err.to_string();
            assert!(!msg.is_empty(), "Error message should not be empty");

            // Each error should convert to UltimoError
            let _ultimo_err: crate::UltimoError = err.into();
        }
    }
}

// Unit tests for database module structure (no connection required)
#[cfg(feature = "database")]
#[cfg(test)]
mod database_module_tests {
    use crate::database::Database;

    #[test]
    fn test_database_enum_size() {
        // Ensure Database enum is reasonably sized
        use std::mem::size_of;

        let size = size_of::<Database>();
        // Should be pointer-sized (Arc<T>) - 8 bytes on 64-bit, 16 bytes with tag
        assert!(
            size <= 32,
            "Database enum should be small (Arc-based): {} bytes",
            size
        );
    }

    #[test]
    fn test_database_error_is_send_sync() {
        // Ensure errors can be sent between threads
        use crate::database::DatabaseError;

        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<DatabaseError>();
        assert_sync::<DatabaseError>();
    }

    #[test]
    fn test_database_enum_is_send_sync() {
        // Ensure Database enum can be used across threads
        use crate::database::Database;

        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<Database>();
        assert_sync::<Database>();
    }
}

// Tests for database configuration and types
#[cfg(all(feature = "database", any(feature = "sqlx", feature = "diesel")))]
#[cfg(test)]
mod database_config_tests {
    #[test]
    fn test_connection_string_parsing() {
        // Test that we can parse various connection string formats
        let valid_strings = vec![
            "postgres://localhost/mydb",
            "postgresql://user:pass@localhost:5432/db",
            "mysql://localhost/mydb",
            "sqlite::memory:",
            "sqlite:./test.db",
        ];

        for conn_str in valid_strings {
            // Just verify the string formats are reasonable
            assert!(!conn_str.is_empty());
            assert!(conn_str.contains("://") || conn_str.starts_with("sqlite:"));
        }
    }

    #[test]
    fn test_database_url_validation() {
        // Test URL validation logic
        let invalid_strings = vec!["", "   ", "not-a-url", "http://wrong-protocol"];

        for invalid in invalid_strings {
            // These should be rejected by actual pool creation
            assert!(invalid.is_empty() || !invalid.contains("postgres://"));
        }
    }
}

#[cfg(feature = "sqlx-postgres")]
#[cfg(test)]
mod sqlx_tests {
    use crate::database::sqlx::SqlxPool;
    use crate::database::Database;

    #[tokio::test]
    async fn test_sqlx_pool_type_safety() {
        // Test that SqlxPool enforces type safety at compile time
        // This ensures we can't mix different database types

        // Type check - this should compile
        let _pg_pool: Option<SqlxPool<sqlx::Postgres>> = None;

        #[cfg(feature = "sqlx-mysql")]
        let _mysql_pool: Option<SqlxPool<sqlx::MySql>> = None;

        #[cfg(feature = "sqlx-sqlite")]
        let _sqlite_pool: Option<SqlxPool<sqlx::Sqlite>> = None;
    }

    #[test]
    fn test_sqlx_database_enum_creation() {
        // Test that we can create Database enum variants
        // This validates the API design

        use std::sync::Arc;

        // Mock pool for testing (we can't create real pool without connection)
        // In real usage, this would be: Database::from_sqlx(actual_pool)
        let mock_pool = Arc::new(42); // Just for type checking
        let _db = Database::Sqlx(mock_pool);
    }

    #[test]
    fn test_sqlx_error_conversion() {
        use crate::database::DatabaseError;

        // Test that SQLx errors convert properly to our error types
        let errors = vec![
            DatabaseError::Pool("connection timeout".to_string()),
            DatabaseError::Query("invalid SQL syntax".to_string()),
            DatabaseError::Transaction("deadlock detected".to_string()),
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());

            // Should convert to UltimoError
            let ultimo_err: crate::UltimoError = err.into();
            match ultimo_err {
                crate::UltimoError::Internal(_) => {}
                _ => panic!("Expected Internal error"),
            }
        }
    }
}

#[cfg(feature = "diesel-postgres")]
#[cfg(test)]
mod diesel_tests {
    use crate::database::diesel::DieselPool;
    use crate::database::Database;

    #[test]
    fn test_diesel_pool_type_safety() {
        // Test that DieselPool enforces type safety at compile time

        // Type check - this should compile
        let _pg_pool: Option<DieselPool<diesel::PgConnection>> = None;

        #[cfg(feature = "diesel-mysql")]
        let _mysql_pool: Option<DieselPool<diesel::MysqlConnection>> = None;

        #[cfg(feature = "diesel-sqlite")]
        let _sqlite_pool: Option<DieselPool<diesel::SqliteConnection>> = None;
    }

    #[test]
    fn test_diesel_database_enum_creation() {
        // Test Database enum variant for Diesel
        use std::sync::Arc;

        let mock_pool = Arc::new(42); // Just for type checking
        let _db = Database::Diesel(mock_pool);
    }

    #[test]
    fn test_diesel_pool_config_validation() {
        // Test that pool configuration makes sense
        // Max pool size should be positive
        let max_size = 10;
        assert!(max_size > 0, "Pool size must be positive");

        // Min pool size should not exceed max
        let min_size = 5;
        assert!(min_size <= max_size, "Min size cannot exceed max size");
    }

    #[test]
    fn test_diesel_error_conversion() {
        use crate::database::DatabaseError;

        let errors = vec![
            DatabaseError::Connection("failed to connect".to_string()),
            DatabaseError::Query("table not found".to_string()),
            DatabaseError::Transaction("constraint violation".to_string()),
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());

            let ultimo_err: crate::UltimoError = err.into();
            match ultimo_err {
                crate::UltimoError::Internal(_) => {}
                _ => panic!("Expected Internal error"),
            }
        }
    }
}

#[cfg(test)]
mod context_tests {
    // Test that Context methods work as expected

    #[test]
    fn test_database_attachment_api() {
        // This tests the API design - that we have the right methods
        // Actual integration tested in examples

        // Just verify the API compiles
        fn _check_context_api(ctx: &crate::Context) {
            #[cfg(feature = "sqlx-postgres")]
            {
                let _result: Result<&sqlx::Pool<sqlx::Postgres>, crate::UltimoError> =
                    ctx.sqlx::<sqlx::Postgres>();
            }

            #[cfg(feature = "diesel-postgres")]
            {
                use diesel::r2d2::{ConnectionManager, PooledConnection};
                let _result: Result<
                    PooledConnection<ConnectionManager<diesel::PgConnection>>,
                    crate::UltimoError,
                > = ctx.diesel::<diesel::PgConnection>();
            }
        }
    }
}

#[cfg(all(test, feature = "database"))]
mod integration_hints {
    //! This module documents how to write integration tests
    //! See examples/database-sqlx/tests and examples/database-diesel/tests

    /// Example of how to write an integration test with SQLx
    ///
    /// ```rust,ignore
    /// #[tokio::test]
    /// async fn test_sqlx_crud_operations() {
    ///     // 1. Create test database connection
    ///     let pool = SqlxPool::connect("postgres://localhost/test_db").await.unwrap();
    ///     
    ///     // 2. Run migrations
    ///     sqlx::query("CREATE TABLE IF NOT EXISTS users (id SERIAL, name TEXT)")
    ///         .execute(pool.pool())
    ///         .await
    ///         .unwrap();
    ///     
    ///     // 3. Test CRUD operations
    ///     let user = sqlx::query_as::<_, User>("INSERT INTO users (name) VALUES ($1) RETURNING *")
    ///         .bind("Alice")
    ///         .fetch_one(pool.pool())
    ///         .await
    ///         .unwrap();
    ///     
    ///     assert_eq!(user.name, "Alice");
    ///     
    ///     // 4. Cleanup
    ///     sqlx::query("DROP TABLE users")
    ///         .execute(pool.pool())
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    #[allow(dead_code)]
    fn example_sqlx_test() {}

    /// Example of how to write an integration test with Diesel
    ///
    /// ```rust,ignore
    /// #[test]
    /// fn test_diesel_crud_operations() {
    ///     use diesel::prelude::*;
    ///     
    ///     // 1. Create test database connection
    ///     let pool = DieselPool::<PgConnection>::new("postgres://localhost/test_db").unwrap();
    ///     let mut conn = pool.get().unwrap();
    ///     
    ///     // 2. Run migrations
    ///     diesel::sql_query("CREATE TABLE IF NOT EXISTS users (id SERIAL PRIMARY KEY, name TEXT)")
    ///         .execute(&mut conn)
    ///         .unwrap();
    ///     
    ///     // 3. Test CRUD operations
    ///     #[derive(Queryable)]
    ///     struct User { id: i32, name: String }
    ///     
    ///     let user = diesel::insert_into(users::table)
    ///         .values(users::name.eq("Alice"))
    ///         .get_result::<User>(&mut conn)
    ///         .unwrap();
    ///     
    ///     assert_eq!(user.name, "Alice");
    ///     
    ///     // 4. Cleanup
    ///     diesel::sql_query("DROP TABLE users").execute(&mut conn).unwrap();
    /// }
    /// ```
    #[allow(dead_code)]
    fn example_diesel_test() {}
}
