use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use ultimo::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

type UserStore = Arc<Mutex<Vec<User>>>;

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    println!("🚀 Ultimo OpenAPI Demo - Regular REST Mode");
    println!();

    // Initialize user store
    let users: UserStore = Arc::new(Mutex::new(vec![
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
    ]));

    let mut app = Ultimo::new();

    // Add CORS middleware
    app.use_middleware(ultimo::middleware::builtin::cors());

    // Add logger middleware
    app.use_middleware(ultimo::middleware::builtin::logger());

    // Regular REST endpoints - no RPC
    let users_get = users.clone();
    app.get("/api/users/:id", move |ctx: Context| {
        let users = users_get.clone();
        async move {
            let id_str = ctx.req.param("id")?;
            let id: u32 = id_str
                .parse()
                .map_err(|_| UltimoError::BadRequest("Invalid 'id' parameter".to_string()))?;

            let user = {
                let users_data = users.lock().unwrap();
                users_data.iter().find(|u| u.id == id).cloned()
            };

            let user = user.ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;

            ctx.json(user).await
        }
    });

    let users_list = users.clone();
    app.get("/api/users", move |ctx: Context| {
        let users = users_list.clone();
        async move {
            let users_data = users.lock().unwrap().clone();
            ctx.json(users_data).await
        }
    });

    let users_create = users.clone();
    app.post("/api/users", move |ctx: Context| {
        let users = users_create.clone();
        async move {
            let input: CreateUserInput = ctx.req.json().await?;
            let new_user = {
                let mut users_data = users.lock().unwrap();
                let new_id = users_data.iter().map(|u| u.id).max().unwrap_or(0) + 1;
                let new_user = User {
                    id: new_id,
                    name: input.name,
                    email: input.email,
                };
                users_data.push(new_user.clone());
                new_user
            };
            ctx.json(new_user).await
        }
    });

    // DELETE /api/users/:id
    let users_delete = users.clone();
    app.delete("/api/users/:id", move |ctx: Context| {
        let users = users_delete.clone();
        async move {
            let id: u32 = ctx
                .req
                .param("id")?
                .parse()
                .map_err(|_| UltimoError::BadRequest("Invalid user ID".to_string()))?;

            {
                let mut users_data = users.lock().unwrap();
                let index = users_data
                    .iter()
                    .position(|u| u.id == id)
                    .ok_or_else(|| UltimoError::NotFound("User not found".to_string()))?;
                users_data.remove(index);
            }

            ctx.status(204).await;
            ctx.text("").await
        }
    });

    // Manually create OpenAPI spec for REST endpoints
    use std::collections::HashMap;
    use ultimo::openapi::{
        MediaType, OpenApiBuilder, Operation, Parameter, ParameterLocation, PathItem, RequestBody,
        Response, Schema,
    };

    println!("📋 Generating OpenAPI specification...");
    let mut openapi = OpenApiBuilder::new()
        .title("User API - REST Mode")
        .version("1.0.0")
        .description("User management API using regular REST endpoints")
        .server(
            "http://127.0.0.1:3000",
            Some("Development server".to_string()),
        )
        .tag("users", Some("User management operations".to_string()))
        .build();

    // GET /api/users/:id
    let mut get_user_op = Operation {
        summary: Some("Get user by ID".to_string()),
        description: Some("Retrieve a single user by their ID".to_string()),
        operation_id: Some("getUser".to_string()),
        tags: Some(vec!["users".to_string()]),
        parameters: Some(vec![Parameter {
            name: "id".to_string(),
            location: ParameterLocation::Path,
            description: Some("User ID".to_string()),
            required: Some(true),
            schema: Schema {
                schema_type: Some("integer".to_string()),
                format: Some("int32".to_string()),
                properties: None,
                required: None,
                items: None,
                reference: None,
            },
        }]),
        request_body: None,
        responses: HashMap::new(),
    };
    get_user_op.responses.insert(
        "200".to_string(),
        Response {
            description: "Successful response".to_string(),
            content: Some({
                let mut content = HashMap::new();
                content.insert(
                    "application/json".to_string(),
                    MediaType {
                        schema: Schema {
                            schema_type: Some("object".to_string()),
                            format: None,
                            properties: Some({
                                let mut props = HashMap::new();
                                props.insert(
                                    "id".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("integer".to_string()),
                                        format: Some("int32".to_string()),
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props.insert(
                                    "name".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("string".to_string()),
                                        format: None,
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props.insert(
                                    "email".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("string".to_string()),
                                        format: Some("email".to_string()),
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props
                            }),
                            required: Some(vec![
                                "id".to_string(),
                                "name".to_string(),
                                "email".to_string(),
                            ]),
                            items: None,
                            reference: None,
                        },
                        example: None,
                    },
                );
                content
            }),
        },
    );
    get_user_op.responses.insert(
        "404".to_string(),
        Response {
            description: "User not found".to_string(),
            content: None,
        },
    );

    openapi.add_path(
        "/api/users/{id}".to_string(),
        PathItem {
            get: Some(get_user_op),
            post: None,
            put: None,
            delete: None,
            patch: None,
        },
    );

    // GET /api/users
    let mut list_users_op = Operation {
        summary: Some("List all users".to_string()),
        description: Some("Retrieve a list of all users".to_string()),
        operation_id: Some("listUsers".to_string()),
        tags: Some(vec!["users".to_string()]),
        parameters: None,
        request_body: None,
        responses: HashMap::new(),
    };
    list_users_op.responses.insert(
        "200".to_string(),
        Response {
            description: "Successful response".to_string(),
            content: Some({
                let mut content = HashMap::new();
                content.insert(
                    "application/json".to_string(),
                    MediaType {
                        schema: Schema {
                            schema_type: Some("array".to_string()),
                            format: None,
                            properties: None,
                            required: None,
                            items: Some(Box::new(Schema {
                                schema_type: Some("object".to_string()),
                                format: None,
                                properties: Some({
                                    let mut props = HashMap::new();
                                    props.insert(
                                        "id".to_string(),
                                        Box::new(Schema {
                                            schema_type: Some("integer".to_string()),
                                            format: Some("int32".to_string()),
                                            properties: None,
                                            required: None,
                                            items: None,
                                            reference: None,
                                        }),
                                    );
                                    props.insert(
                                        "name".to_string(),
                                        Box::new(Schema {
                                            schema_type: Some("string".to_string()),
                                            format: None,
                                            properties: None,
                                            required: None,
                                            items: None,
                                            reference: None,
                                        }),
                                    );
                                    props.insert(
                                        "email".to_string(),
                                        Box::new(Schema {
                                            schema_type: Some("string".to_string()),
                                            format: Some("email".to_string()),
                                            properties: None,
                                            required: None,
                                            items: None,
                                            reference: None,
                                        }),
                                    );
                                    props
                                }),
                                required: Some(vec![
                                    "id".to_string(),
                                    "name".to_string(),
                                    "email".to_string(),
                                ]),
                                items: None,
                                reference: None,
                            })),
                            reference: None,
                        },
                        example: None,
                    },
                );
                content
            }),
        },
    );

    openapi.add_path(
        "/api/users".to_string(),
        PathItem {
            get: Some(list_users_op),
            post: None,
            put: None,
            delete: None,
            patch: None,
        },
    );

    // POST /api/users
    let mut create_user_op = Operation {
        summary: Some("Create a new user".to_string()),
        description: Some("Create a new user with name and email".to_string()),
        operation_id: Some("createUser".to_string()),
        tags: Some(vec!["users".to_string()]),
        parameters: None,
        request_body: Some(RequestBody {
            description: Some("User data".to_string()),
            content: {
                let mut content = HashMap::new();
                content.insert(
                    "application/json".to_string(),
                    MediaType {
                        schema: Schema {
                            schema_type: Some("object".to_string()),
                            format: None,
                            properties: Some({
                                let mut props = HashMap::new();
                                props.insert(
                                    "name".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("string".to_string()),
                                        format: None,
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props.insert(
                                    "email".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("string".to_string()),
                                        format: Some("email".to_string()),
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props
                            }),
                            required: Some(vec!["name".to_string(), "email".to_string()]),
                            items: None,
                            reference: None,
                        },
                        example: None,
                    },
                );
                content
            },
            required: Some(true),
        }),
        responses: HashMap::new(),
    };
    create_user_op.responses.insert(
        "200".to_string(),
        Response {
            description: "User created successfully".to_string(),
            content: Some({
                let mut content = HashMap::new();
                content.insert(
                    "application/json".to_string(),
                    MediaType {
                        schema: Schema {
                            schema_type: Some("object".to_string()),
                            format: None,
                            properties: Some({
                                let mut props = HashMap::new();
                                props.insert(
                                    "id".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("integer".to_string()),
                                        format: Some("int32".to_string()),
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props.insert(
                                    "name".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("string".to_string()),
                                        format: None,
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props.insert(
                                    "email".to_string(),
                                    Box::new(Schema {
                                        schema_type: Some("string".to_string()),
                                        format: Some("email".to_string()),
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    }),
                                );
                                props
                            }),
                            required: Some(vec![
                                "id".to_string(),
                                "name".to_string(),
                                "email".to_string(),
                            ]),
                            items: None,
                            reference: None,
                        },
                        example: None,
                    },
                );
                content
            }),
        },
    );

    // Update the path with POST operation
    if let Some(path_item) = openapi.paths.get_mut("/api/users") {
        path_item.post = Some(create_user_op);
    }

    // DELETE /api/users/:id
    let mut delete_user_op = Operation {
        summary: Some("Delete a user".to_string()),
        description: Some("Delete a user by their ID".to_string()),
        operation_id: Some("deleteUser".to_string()),
        tags: Some(vec!["users".to_string()]),
        parameters: Some(vec![Parameter {
            name: "id".to_string(),
            location: ParameterLocation::Path,
            description: Some("User ID".to_string()),
            required: Some(true),
            schema: Schema {
                schema_type: Some("integer".to_string()),
                format: Some("int32".to_string()),
                properties: None,
                required: None,
                items: None,
                reference: None,
            },
        }]),
        request_body: None,
        responses: HashMap::new(),
    };
    delete_user_op.responses.insert(
        "204".to_string(),
        Response {
            description: "User deleted successfully".to_string(),
            content: None,
        },
    );
    delete_user_op.responses.insert(
        "404".to_string(),
        Response {
            description: "User not found".to_string(),
            content: None,
        },
    );

    // Add DELETE operation to /api/users/:id path
    if let Some(path_item) = openapi.paths.get_mut("/api/users/{id}") {
        path_item.delete = Some(delete_user_op);
    }

    // Save OpenAPI spec
    openapi
        .write_to_file("openapi-rest.json")
        .expect("Failed to write OpenAPI spec");
    println!("✅ OpenAPI spec saved to: openapi-rest.json");
    println!();

    // Serve OpenAPI spec endpoint
    let openapi_clone = openapi.clone();
    app.get("/openapi.json", move |ctx: Context| {
        let spec = openapi_clone.clone();
        async move { ctx.json(spec).await }
    });

    // Serve Swagger UI at /docs
    let openapi_docs = openapi.clone();
    app.get("/docs", move |ctx: Context| {
        let html = openapi_docs.swagger_ui_html("/openapi.json");
        async move { ctx.html(html).await }
    });

    println!("🌐 Server starting on http://127.0.0.1:3000");
    println!();
    println!("Available endpoints:");
    println!("  GET  /api/users/:id");
    println!("  GET  /api/users");
    println!("  POST /api/users");
    println!();
    println!("📖 Interactive API Documentation:");
    println!("  Swagger UI: http://127.0.0.1:3000/docs");
    println!("  OpenAPI:    http://127.0.0.1:3000/openapi.json");
    println!();

    app.listen("127.0.0.1:3000").await
}
