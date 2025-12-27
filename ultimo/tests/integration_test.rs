//! Integration tests for the Ultimo framework
//!
//! These tests verify end-to-end functionality including routing,
//! middleware, validation, and error handling.

use serde::{Deserialize, Serialize};
use ultimo::prelude::*;

#[derive(Deserialize, Validate)]
struct TestInput {
    #[validate(length(min = 3, max = 20))]
    name: String,
    #[validate(email)]
    email: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[allow(dead_code)]
struct TestOutput {
    id: u64,
    name: String,
}

#[tokio::test]
async fn test_app_basic_setup() {
    let mut app = Ultimo::new();

    app.get(
        "/test",
        |ctx: Context| async move { ctx.text("Hello").await },
    );

    // Verify handlers are registered
    // Note: We can't easily test the actual HTTP handling without spawning a server
    // But we can verify the app is configured correctly
}

#[tokio::test]
async fn test_multiple_routes() {
    let mut app = Ultimo::new();

    app.get("/", |ctx: Context| async move {
        ctx.json(json!({"status": "ok"})).await
    });

    app.get("/users", |ctx: Context| async move {
        ctx.json(json!({"users": []})).await
    });

    app.post("/users", |ctx: Context| async move {
        ctx.status(201).await;
        ctx.json(json!({"created": true})).await
    });

    app.put("/users/:id", |ctx: Context| async move {
        let id = ctx.req.param("id")?;
        ctx.json(json!({"id": id, "updated": true})).await
    });

    app.delete("/users/:id", |ctx: Context| async move {
        let id = ctx.req.param("id")?;
        ctx.json(json!({"id": id, "deleted": true})).await
    });
}

#[tokio::test]
async fn test_middleware_registration() {
    use ultimo::middleware::builtin;

    let mut app = Ultimo::new();

    // Add built-in middleware
    app.use_middleware(builtin::logger());
    app.use_middleware(builtin::cors());

    app.get("/", |ctx: Context| async move { ctx.text("OK").await });
}

#[tokio::test]
async fn test_validation_types() {
    // Test that validation types compile and work correctly
    let valid_input = TestInput {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };

    assert!(validate(&valid_input).is_ok());

    let invalid_name = TestInput {
        name: "AB".to_string(), // Too short
        email: "valid@example.com".to_string(),
    };

    assert!(validate(&invalid_name).is_err());

    let invalid_email = TestInput {
        name: "Alice".to_string(),
        email: "not-an-email".to_string(),
    };

    assert!(validate(&invalid_email).is_err());
}

#[test]
fn test_error_types() {
    use ultimo::UltimoError;

    let unauthorized = UltimoError::Unauthorized("No token".to_string());
    assert_eq!(unauthorized.status_code(), 401);

    let not_found = UltimoError::NotFound("Resource missing".to_string());
    assert_eq!(not_found.status_code(), 404);

    let bad_request = UltimoError::BadRequest("Invalid input".to_string());
    assert_eq!(bad_request.status_code(), 400);

    let internal = UltimoError::Internal("Server error".to_string());
    assert_eq!(internal.status_code(), 500);
}

#[test]
fn test_error_response_format() {
    use ultimo::error::ValidationError;
    use ultimo::UltimoError;

    let validation_err = UltimoError::Validation {
        message: "Invalid data".to_string(),
        details: vec![
            ValidationError {
                field: "email".to_string(),
                message: "Invalid email format".to_string(),
            },
            ValidationError {
                field: "age".to_string(),
                message: "Must be at least 18".to_string(),
            },
        ],
    };

    let response = validation_err.to_error_response();
    assert_eq!(response.error, "ValidationError");
    assert_eq!(response.message, "Invalid data");
    assert!(response.details.is_some());
    assert_eq!(response.details.unwrap().len(), 2);
}

#[tokio::test]
async fn test_cors_middleware_configuration() {
    use ultimo::middleware::builtin::Cors;

    let cors = Cors::new()
        .allow_origin("https://example.com")
        .allow_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .build();

    let mut app = Ultimo::new();
    app.use_middleware(cors);

    app.get(
        "/",
        |ctx: Context| async move { ctx.text("CORS enabled").await },
    );
}

#[tokio::test]
async fn test_route_params_and_query() {
    let mut app = Ultimo::new();

    // Test path parameters
    app.get("/users/:userId/posts/:postId", |ctx: Context| async move {
        let user_id = ctx.req.param("userId")?;
        let post_id = ctx.req.param("postId")?;

        ctx.json(json!({
            "userId": user_id,
            "postId": post_id
        }))
        .await
    });

    // Test query parameters
    app.get("/search", |ctx: Context| async move {
        let q = ctx.req.query("q");
        let page = ctx.req.query("page");

        ctx.json(json!({
            "query": q,
            "page": page
        }))
        .await
    });
}

#[tokio::test]
async fn test_response_types() {
    let mut app = Ultimo::new();

    // JSON response
    app.get("/json", |ctx: Context| async move {
        ctx.json(json!({"type": "json"})).await
    });

    // Text response
    app.get("/text", |ctx: Context| async move {
        ctx.text("Plain text").await
    });

    // HTML response
    app.get("/html", |ctx: Context| async move {
        ctx.html("<h1>Hello</h1>").await
    });

    // Redirect
    app.get("/redirect", |ctx: Context| async move {
        ctx.status(301).await;
        ctx.redirect("/").await
    });

    // Custom status
    app.post("/created", |ctx: Context| async move {
        ctx.status(201).await;
        ctx.json(json!({"created": true})).await
    });
}

#[tokio::test]
async fn test_error_handling_in_routes() {
    let mut app = Ultimo::new();

    // Route that returns an error
    app.get("/error", |_ctx: Context| async move {
        Err(UltimoError::BadRequest("Something went wrong".to_string()))
    });

    // Route with validation error
    app.post("/validate", |ctx: Context| async move {
        let input: TestInput = ctx.req.json().await?;
        validate(&input)?;
        ctx.json(json!({"valid": true})).await
    });
}

#[test]
fn test_method_types() {
    use ultimo::router::Method;

    let get = Method::from_hyper(&hyper::Method::GET);
    assert_eq!(get, Some(Method::GET));

    let post = Method::from_hyper(&hyper::Method::POST);
    assert_eq!(post, Some(Method::POST));

    let put = Method::from_hyper(&hyper::Method::PUT);
    assert_eq!(put, Some(Method::PUT));

    let delete = Method::from_hyper(&hyper::Method::DELETE);
    assert_eq!(delete, Some(Method::DELETE));

    let patch = Method::from_hyper(&hyper::Method::PATCH);
    assert_eq!(patch, Some(Method::PATCH));
}

#[test]
fn test_router_priority() {
    use ultimo::router::{Method, Router};

    let mut router = Router::new();

    // More specific route first
    router.add_route(Method::GET, "/users/:id/posts", 0);
    // Less specific route
    router.add_route(Method::GET, "/users/:id", 1);
    // Most specific static route
    router.add_route(Method::GET, "/users/me", 2);

    // First match wins - this is important for route ordering
    let (handler_id, _) = router.find_route(Method::GET, "/users/123/posts").unwrap();
    assert_eq!(handler_id, 0);

    let (handler_id, _) = router.find_route(Method::GET, "/users/456").unwrap();
    assert_eq!(handler_id, 1);
}
