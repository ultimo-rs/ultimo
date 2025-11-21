use serde::{Deserialize, Serialize};
use ultimo::openapi::OpenApiBuilder;
use ultimo::prelude::*;
use ultimo::rpc::RpcMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct GetUserInput {
    id: u32,
}

#[derive(Debug, Deserialize)]
struct CreateUserInput {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct EmptyParams {}

#[tokio::main]
async fn main() -> ultimo::Result<()> {
    println!("🚀 Ultimo OpenAPI Demo");
    println!();

    // ============================================
    // Example 1: OpenAPI from RPC Registry (REST Mode)
    // ============================================
    println!("📋 Generating OpenAPI spec from RPC registry (REST mode)...");

    let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);

    // Register procedures with TypeScript types
    rpc.query(
        "getUser",
        |input: GetUserInput| async move {
            Ok(User {
                id: input.id,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            })
        },
        "{ id: number }".to_string(),
        "{ id: number; name: string; email: string }".to_string(),
    );

    rpc.query(
        "listUsers",
        |_input: EmptyParams| async move {
            Ok(vec![
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
            ])
        },
        "{}".to_string(),
        "{ id: number; name: string; email: string }[]".to_string(),
    );

    rpc.mutation(
        "createUser",
        |input: CreateUserInput| async move {
            Ok(User {
                id: 3,
                name: input.name,
                email: input.email,
            })
        },
        "{ name: string; email: string }".to_string(),
        "{ id: number; name: string; email: string }".to_string(),
    );

    // Generate OpenAPI spec from RPC registry
    let openapi_rest = rpc.generate_openapi("User API (REST Mode)", "1.0.0", "/api");

    // Save to file
    openapi_rest
        .write_to_file("openapi-rest.json")
        .expect("Failed to write OpenAPI spec");

    println!("✅ Generated: openapi-rest.json");
    println!("   - GET  /api/getUser");
    println!("   - GET  /api/listUsers");
    println!("   - POST /api/createUser");
    println!();

    // ============================================
    // Example 2: OpenAPI from RPC Registry (JSON-RPC Mode)
    // ============================================
    println!("📋 Generating OpenAPI spec from RPC registry (JSON-RPC mode)...");

    let rpc_jsonrpc = RpcRegistry::new(); // Default is JsonRpc mode

    // Register same procedures
    rpc_jsonrpc.register_with_types(
        "getUser",
        |input: GetUserInput| async move {
            Ok(User {
                id: input.id,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            })
        },
        "{ id: number }".to_string(),
        "{ id: number; name: string; email: string }".to_string(),
    );

    rpc_jsonrpc.register_with_types(
        "listUsers",
        |_input: EmptyParams| async move {
            Ok(vec![User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            }])
        },
        "{}".to_string(),
        "{ id: number; name: string; email: string }[]".to_string(),
    );

    // Generate OpenAPI spec
    let openapi_jsonrpc = rpc_jsonrpc.generate_openapi("User API (JSON-RPC Mode)", "1.0.0", "/rpc");

    openapi_jsonrpc
        .write_to_file("openapi-jsonrpc.json")
        .expect("Failed to write OpenAPI spec");

    println!("✅ Generated: openapi-jsonrpc.json");
    println!("   - POST /rpc (with method dispatch)");
    println!();

    // ============================================
    // Example 3: Manual OpenAPI Builder
    // ============================================
    println!("📋 Building OpenAPI spec manually...");

    let manual_spec = OpenApiBuilder::new()
        .title("Custom API")
        .version("2.0.0")
        .description("Manually built OpenAPI specification")
        .server(
            "https://api.example.com",
            Some("Production server".to_string()),
        )
        .server(
            "http://localhost:3000",
            Some("Development server".to_string()),
        )
        .tag("users", Some("User management operations".to_string()))
        .tag("auth", Some("Authentication operations".to_string()))
        .contact(
            Some("API Support".to_string()),
            Some("support@example.com".to_string()),
            Some("https://example.com/support".to_string()),
        )
        .license(
            "MIT",
            Some("https://opensource.org/licenses/MIT".to_string()),
        )
        .build();

    manual_spec
        .write_to_file("openapi-manual.json")
        .expect("Failed to write OpenAPI spec");

    println!("✅ Generated: openapi-manual.json");
    println!();

    // ============================================
    // Usage Instructions
    // ============================================
    println!("🎯 Next Steps:");
    println!();
    println!("1. View in Swagger UI:");
    println!("   docker run -p 8080:8080 -e SWAGGER_JSON=/openapi.json \\");
    println!("     -v $(pwd)/openapi-rest.json:/openapi.json swaggerapi/swagger-ui");
    println!("   Then open: http://localhost:8080");
    println!();
    println!("2. Run Prism Mock Server:");
    println!("   npx @stoplight/prism-cli mock openapi-rest.json");
    println!();
    println!("3. Generate TypeScript Client:");
    println!("   npx @openapitools/openapi-generator-cli generate \\");
    println!("     -i openapi-rest.json \\");
    println!("     -g typescript-fetch \\");
    println!("     -o ./generated-client");
    println!();
    println!("4. Generate Python Client:");
    println!("   npx @openapitools/openapi-generator-cli generate \\");
    println!("     -i openapi-rest.json \\");
    println!("     -g python \\");
    println!("     -o ./generated-client-py");
    println!();
    println!("5. Validate OpenAPI Spec:");
    println!("   npx @apidevtools/swagger-cli validate openapi-rest.json");

    Ok(())
}
