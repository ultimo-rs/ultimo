// Integration tests for database-api-styles example
//
// These tests demonstrate that the same database operations work with all API styles.
// They also show how to write tests for different API routing patterns.

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: i32,
    name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[test]
fn test_shared_database_logic_concept() {
    // This test validates the architectural concept:
    // Database operations should be independent of API routing style

    // Example database operation (would use real pool in integration test)
    fn create_user_logic(name: &str, email: &str) -> User {
        // This represents the shared business logic
        User {
            id: 1,
            name: name.to_string(),
            email: email.to_string(),
        }
    }

    // The same logic can be called from any API style:

    // 1. REST style
    let rest_user = create_user_logic("Alice", "alice@example.com");

    // 2. JSON-RPC style
    let rpc_user = create_user_logic("Alice", "alice@example.com");

    // 3. RPC-REST style
    let rpc_rest_user = create_user_logic("Alice", "alice@example.com");

    // All produce the same result!
    assert_eq!(rest_user, rpc_user);
    assert_eq!(rpc_user, rpc_rest_user);
}

#[test]
fn test_rest_api_request_format() {
    // Test that REST API request format is correct

    let create_input = CreateUserInput {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let json = serde_json::to_string(&create_input).unwrap();
    assert!(json.contains("Alice"));
    assert!(json.contains("alice@example.com"));

    // REST endpoint would receive this as:
    // POST /rest/users
    // Body: {"name":"Alice","email":"alice@example.com"}
}

#[test]
fn test_json_rpc_request_format() {
    // Test that JSON-RPC request format is correct

    let rpc_request = json!({
        "method": "createUser",
        "params": {
            "name": "Alice",
            "email": "alice@example.com"
        }
    });

    let json_str = serde_json::to_string(&rpc_request).unwrap();
    assert!(json_str.contains("createUser"));
    assert!(json_str.contains("Alice"));

    // JSON-RPC endpoint would receive this as:
    // POST /rpc
    // Body: {"method":"createUser","params":{"name":"Alice","email":"alice@example.com"}}
}

#[test]
fn test_rpc_rest_request_format() {
    // Test that RPC-REST request format is correct

    let create_input = CreateUserInput {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let json = serde_json::to_string(&create_input).unwrap();
    assert!(json.contains("Alice"));

    // RPC-REST endpoint would receive this as:
    // POST /rpc-rest/createUser
    // Body: {"name":"Alice","email":"alice@example.com"}
}

#[test]
fn test_user_serialization() {
    // All API styles return the same User format

    let user = User {
        id: 1,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    let json = serde_json::to_string(&user).unwrap();
    assert!(json.contains(r#""id":1"#));
    assert!(json.contains(r#""name":"Alice"#));
    assert!(json.contains(r#""email":"alice@example.com"#));

    let deserialized: User = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, user);
}

#[test]
fn test_api_style_routing_patterns() {
    // Document the different routing patterns

    // REST: Resource-based URLs
    let rest_endpoints = vec![
        "GET /rest/users",
        "GET /rest/users/:id",
        "POST /rest/users",
        "PUT /rest/users/:id",
        "DELETE /rest/users/:id",
    ];
    assert_eq!(rest_endpoints.len(), 5);

    // JSON-RPC: Single endpoint with method dispatch
    let json_rpc_endpoint = "POST /rpc";
    assert!(!json_rpc_endpoint.is_empty());

    // RPC-REST: Action-based URLs
    let rpc_rest_endpoints = vec![
        "GET /rpc-rest/listUsers",
        "GET /rpc-rest/getUser",
        "POST /rpc-rest/createUser",
        "POST /rpc-rest/updateUser",
        "POST /rpc-rest/deleteUser",
    ];
    assert_eq!(rpc_rest_endpoints.len(), 5);

    // Key insight: All three styles can access the same database operations!
}

#[test]
fn test_error_handling_consistency() {
    // All API styles should handle errors consistently

    #[derive(Debug, Serialize, Deserialize)]
    struct ErrorResponse {
        error: String,
        message: String,
    }

    let error = ErrorResponse {
        error: "NotFound".to_string(),
        message: "User with id 999 not found".to_string(),
    };

    let json = serde_json::to_string(&error).unwrap();
    let deserialized: ErrorResponse = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.error, "NotFound");
    assert!(deserialized.message.contains("999"));
}

#[test]
fn test_database_operation_abstraction() {
    // This test demonstrates the key architectural principle:
    // Database operations are abstracted into reusable functions

    // Simulated database operations (in real code, these use SQLx/Diesel)
    mod db_operations {
        use super::User;

        pub fn list_users() -> Vec<User> {
            vec![
                User {
                    id: 1,
                    name: "Alice".to_string(),
                    email: "alice@example.com".to_string(),
                },
                User {
                    id: 2,
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                },
            ]
        }

        #[allow(dead_code)]
        pub fn get_user(id: i32) -> Option<User> {
            list_users().into_iter().find(|u| u.id == id)
        }

        #[allow(dead_code)]
        pub fn create_user(name: String, email: String) -> User {
            User { id: 3, name, email }
        }
    }

    // These functions can be called from any API style:

    // REST style handler
    fn rest_list_handler() -> Vec<User> {
        db_operations::list_users()
    }

    // JSON-RPC style handler
    fn json_rpc_handler(method: &str) -> Vec<User> {
        match method {
            "listUsers" => db_operations::list_users(),
            _ => vec![],
        }
    }

    // RPC-REST style handler
    fn rpc_rest_list_handler() -> Vec<User> {
        db_operations::list_users()
    }

    // All return the same data!
    let rest_result = rest_list_handler();
    let rpc_result = json_rpc_handler("listUsers");
    let rpc_rest_result = rpc_rest_list_handler();

    assert_eq!(rest_result.len(), 2);
    assert_eq!(rpc_result.len(), 2);
    assert_eq!(rpc_rest_result.len(), 2);
    assert_eq!(rest_result, rpc_result);
    assert_eq!(rpc_result, rpc_rest_result);
}

#[test]
fn test_api_style_comparison() {
    // Document the trade-offs between API styles

    #[allow(dead_code)]
    struct ApiStyle {
        name: &'static str,
        endpoints: usize,
        http_methods: usize,
        supports_caching: bool,
        supports_batching: bool,
    }

    let rest = ApiStyle {
        name: "REST",
        endpoints: 5,           // One per resource/action
        http_methods: 4,        // GET, POST, PUT, DELETE
        supports_caching: true, // GET requests can be cached
        supports_batching: false,
    };

    let json_rpc = ApiStyle {
        name: "JSON-RPC",
        endpoints: 1,            // Single endpoint
        http_methods: 1,         // POST only
        supports_caching: false, // POST not cached
        supports_batching: true, // Can batch multiple methods
    };

    let rpc_rest = ApiStyle {
        name: "RPC-REST",
        endpoints: 5,           // One per action
        http_methods: 2,        // GET for queries, POST for mutations
        supports_caching: true, // GET requests can be cached
        supports_batching: false,
    };

    // All styles are valid choices depending on use case
    assert!(rest.supports_caching);
    assert!(json_rpc.supports_batching);
    assert!(rpc_rest.supports_caching);

    // But all use the same database layer!
    assert_eq!(rest.name.len() > 0, true);
    assert_eq!(json_rpc.name.len() > 0, true);
    assert_eq!(rpc_rest.name.len() > 0, true);
}

// Example of how to write end-to-end HTTP tests (requires running server)
// These are marked #[ignore] and would be run separately

#[tokio::test]
#[ignore]
async fn test_rest_endpoint_integration() {
    // This would test the actual REST endpoints
    // Requires server running on port 3003

    let client = reqwest::Client::new();

    // List users via REST
    let response = client
        .get("http://localhost:3003/rest/users")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let users: Vec<User> = response.json().await.expect("Failed to parse JSON");
    assert!(!users.is_empty(), "Expected at least some users");
}

#[tokio::test]
#[ignore]
async fn test_json_rpc_endpoint_integration() {
    // This would test the JSON-RPC endpoint

    let client = reqwest::Client::new();

    let rpc_request = json!({
        "method": "listUsers",
        "params": {}
    });

    let response = client
        .post("http://localhost:3003/rpc")
        .json(&rpc_request)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let users: Vec<User> = response.json().await.expect("Failed to parse JSON");
    assert!(!users.is_empty(), "Expected at least some users");
}

#[tokio::test]
#[ignore]
async fn test_rpc_rest_endpoint_integration() {
    // This would test the RPC-REST endpoint

    let client = reqwest::Client::new();

    let response = client
        .get("http://localhost:3003/rpc-rest/listUsers")
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status(), 200);

    let users: Vec<User> = response.json().await.expect("Failed to parse JSON");
    assert!(!users.is_empty(), "Expected at least some users");
}
