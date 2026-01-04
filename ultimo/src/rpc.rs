//! RPC (Remote Procedure Call) system for Ultimo
//!
//! Provides type-safe RPC functionality with automatic TypeScript client generation.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// RPC mode determines how procedures are exposed as HTTP endpoints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RpcMode {
    /// Single JSON-RPC endpoint: POST /rpc with {"method": "...", "params": {}}
    ///
    /// Best for:
    /// - Internal APIs and microservices
    /// - When you need request batching
    /// - Simple routing requirements
    /// - When all RPCs need same middleware
    #[default]
    JsonRpc,

    /// Individual REST-like endpoints: GET /api/getUser, POST /api/createUser
    ///
    /// Best for:
    /// - Public APIs
    /// - When debugging is important (clear URLs in Network tab)
    /// - When you need HTTP caching (GET requests)
    /// - RESTful conventions
    Rest,
}

/// RPC procedure handler
pub type RpcHandler = Arc<dyn Fn(serde_json::Value) -> RpcHandlerFuture + Send + Sync>;

/// Future returned by RPC handlers
pub type RpcHandlerFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value>> + Send>>;

/// RPC registry for managing procedures
#[derive(Clone)]
pub struct RpcRegistry {
    mode: RpcMode,
    procedures: Arc<std::sync::Mutex<HashMap<String, RpcHandler>>>,
    type_definitions: Arc<std::sync::Mutex<Vec<TypeDefinition>>>,
    metadata: Arc<std::sync::Mutex<HashMap<String, ProcedureMetadata>>>,
}

/// Type definition for TypeScript generation
#[derive(Debug, Clone, Serialize)]
pub struct TypeDefinition {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub ts_input: String,
    pub ts_output: String,
}

/// Metadata about an RPC procedure
#[derive(Debug, Clone)]
pub struct ProcedureMetadata {
    pub name: String,
    pub is_query: bool, // true = GET (idempotent), false = POST (mutation)
}

impl RpcRegistry {
    /// Create a new RPC registry with default JsonRpc mode
    pub fn new() -> Self {
        Self::new_with_mode(RpcMode::JsonRpc)
    }

    /// Create a new RPC registry with specified mode
    pub fn new_with_mode(mode: RpcMode) -> Self {
        Self {
            mode,
            procedures: Arc::new(std::sync::Mutex::new(HashMap::new())),
            type_definitions: Arc::new(std::sync::Mutex::new(Vec::new())),
            metadata: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Get the current RPC mode
    pub fn mode(&self) -> RpcMode {
        self.mode
    }

    /// Register an RPC procedure with optional TypeScript type information
    pub fn register<F, Fut, I, O>(&self, name: impl Into<String>, handler: F)
    where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + 'static,
        O: Serialize + 'static,
    {
        self.register_with_types(name, handler, "any".to_string(), "any".to_string())
    }

    /// Register an RPC procedure with explicit TypeScript types
    pub fn register_with_types<F, Fut, I, O>(
        &self,
        name: impl Into<String>,
        handler: F,
        ts_input: String,
        ts_output: String,
    ) where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + 'static,
        O: Serialize + 'static,
    {
        self.procedure(name, handler, ts_input, ts_output, false)
    }

    /// Register a query procedure (idempotent, uses GET in REST mode)
    pub fn query<F, Fut, I, O>(
        &self,
        name: impl Into<String>,
        handler: F,
        ts_input: String,
        ts_output: String,
    ) where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + 'static,
        O: Serialize + 'static,
    {
        self.procedure(name, handler, ts_input, ts_output, true)
    }

    /// Register a mutation procedure (state-changing, uses POST in REST mode)
    pub fn mutation<F, Fut, I, O>(
        &self,
        name: impl Into<String>,
        handler: F,
        ts_input: String,
        ts_output: String,
    ) where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + 'static,
        O: Serialize + 'static,
    {
        self.procedure(name, handler, ts_input, ts_output, false)
    }

    /// Internal method to register with options
    fn procedure<F, Fut, I, O>(
        &self,
        name: impl Into<String>,
        handler: F,
        ts_input: String,
        ts_output: String,
        is_query: bool,
    ) where
        F: Fn(I) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<O>> + Send + 'static,
        I: for<'de> Deserialize<'de> + 'static,
        O: Serialize + 'static,
    {
        let name = name.into();
        let name_clone = name.clone();

        let wrapped_handler: RpcHandler = Arc::new(move |input| {
            let handler = handler.clone();
            Box::pin(async move {
                let input: I = serde_json::from_value(input)
                    .map_err(|e| crate::UltimoError::BadRequest(format!("Invalid input: {}", e)))?;
                let output = handler(input).await?;
                serde_json::to_value(output).map_err(|e| {
                    crate::UltimoError::Internal(format!("Serialization error: {}", e))
                })
            })
        });

        self.procedures
            .lock()
            .unwrap()
            .insert(name.clone(), wrapped_handler);

        // Store type definition metadata
        let type_def = TypeDefinition {
            name: name_clone.clone(),
            input_type: std::any::type_name::<I>().to_string(),
            output_type: std::any::type_name::<O>().to_string(),
            ts_input,
            ts_output,
        };

        self.type_definitions.lock().unwrap().push(type_def);

        // Store procedure metadata
        let metadata = ProcedureMetadata {
            name: name_clone.clone(),
            is_query,
        };

        self.metadata.lock().unwrap().insert(name, metadata);
    }

    /// Call an RPC procedure
    pub async fn call(&self, name: &str, input: serde_json::Value) -> Result<serde_json::Value> {
        let handler = {
            let procedures = self.procedures.lock().unwrap();
            procedures
                .get(name)
                .ok_or_else(|| {
                    crate::UltimoError::NotFound(format!("Procedure '{}' not found", name))
                })?
                .clone()
        };

        handler(input).await
    }

    /// Get all registered procedure names
    pub fn list_procedures(&self) -> Vec<String> {
        self.procedures.lock().unwrap().keys().cloned().collect()
    }

    /// Generate TypeScript client code
    pub fn generate_typescript_client(&self) -> String {
        match self.mode {
            RpcMode::JsonRpc => self.generate_json_rpc_client(),
            RpcMode::Rest => self.generate_rest_client(),
        }
    }

    /// Generate JSON-RPC style client (single endpoint)
    fn generate_json_rpc_client(&self) -> String {
        let type_defs = self.type_definitions.lock().unwrap();

        let mut client = String::from(
            r#"// Auto-generated TypeScript client for Ultimo RPC (JSON-RPC Mode)
// DO NOT EDIT - This file is automatically generated

export class UltimoRpcClient {
  constructor(private baseUrl: string = '/api/rpc') {}

  private async call<T>(method: string, params: any): Promise<T> {
    const response = await fetch(this.baseUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ method, params }),
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || 'RPC call failed');
    }

    const data = await response.json();
    return data.result;
  }

"#,
        );

        // Generate method for each procedure
        for def in type_defs.iter() {
            client.push_str(&format!(
                r#"  async {}(params: {}): Promise<{}> {{
    return this.call('{}', params);
  }}

"#,
                def.name, def.ts_input, def.ts_output, def.name
            ));
        }

        client.push_str("}\n");
        self.append_type_definitions(&mut client);
        client
    }

    /// Generate REST style client (individual endpoints)
    fn generate_rest_client(&self) -> String {
        let type_defs = self.type_definitions.lock().unwrap();
        let metadata = self.metadata.lock().unwrap();

        let mut client = String::from(
            r#"// Auto-generated TypeScript client for Ultimo RPC (REST Mode)
// DO NOT EDIT - This file is automatically generated

export class UltimoRpcClient {
  constructor(private baseUrl: string = '/api') {}

  private async get<T>(path: string, params?: Record<string, any>): Promise<T> {
    const url = new URL(this.baseUrl + path, window.location.origin);
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        url.searchParams.append(key, String(value));
      });
    }

    const response = await fetch(url.toString(), {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ message: response.statusText }));
      throw new Error(error.message || 'Request failed');
    }

    return response.json();
  }

  private async post<T>(path: string, body: any): Promise<T> {
    const response = await fetch(this.baseUrl + path, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({ message: response.statusText }));
      throw new Error(error.message || 'Request failed');
    }

    return response.json();
  }

"#,
        );

        // Generate method for each procedure
        for def in type_defs.iter() {
            let meta = metadata.get(&def.name);
            let is_query = meta.map(|m| m.is_query).unwrap_or(false);

            if is_query {
                // Query: Use GET
                client.push_str(&format!(
                    r#"  async {}(params: {}): Promise<{}> {{
    return this.get('/{}', params);
  }}

"#,
                    def.name, def.ts_input, def.ts_output, def.name,
                ));
            } else {
                // Mutation: Use POST
                client.push_str(&format!(
                    r#"  async {}(params: {}): Promise<{}> {{
    return this.post('/{}'{}params);
  }}

"#,
                    def.name,
                    def.ts_input,
                    def.ts_output,
                    def.name,
                    if def.ts_input == "{}" { "" } else { ", " }
                ));
            }
        }

        client.push_str("}\n");
        self.append_type_definitions(&mut client);
        client
    }

    /// Append type definitions to generated client
    fn append_type_definitions(&self, client: &mut String) {
        client.push_str("\n// Type Definitions\n");
        client.push_str("export interface User {\n");
        client.push_str("  id: number;\n");
        client.push_str("  name: string;\n");
        client.push_str("  email: string;\n");
        client.push_str("}\n");
    }

    /// Generate TypeScript client and save to file
    pub fn generate_client_file(&self, output_path: &str) -> std::io::Result<()> {
        let client_code = self.generate_typescript_client();
        std::fs::write(output_path, client_code)?;
        Ok(())
    }

    /// Generate OpenAPI specification from RPC registry
    ///
    /// # Arguments
    /// * `title` - API title
    /// * `version` - API version
    /// * `base_path` - Base path for REST mode endpoints (e.g., "/api")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ultimo::rpc::{RpcRegistry, RpcMode};
    ///
    /// let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);
    /// // ... register procedures ...
    ///
    /// let spec = rpc.generate_openapi("My API", "1.0.0", "/api");
    /// spec.write_to_file("openapi.json").unwrap();
    /// ```
    pub fn generate_openapi(
        &self,
        title: &str,
        version: &str,
        base_path: &str,
    ) -> crate::openapi::OpenApiSpec {
        use crate::openapi::{
            MediaType, OpenApiBuilder, Operation, Parameter, ParameterLocation, PathItem,
            RequestBody, Response,
        };
        use std::collections::HashMap;

        let mut spec = OpenApiBuilder::new()
            .title(title)
            .version(version)
            .server(
                "http://localhost:3000".to_string(),
                Some("Development server".to_string()),
            )
            .build();

        let type_defs = self.type_definitions.lock().unwrap();
        let metadata = self.metadata.lock().unwrap();

        match self.mode {
            RpcMode::Rest => {
                // Generate individual REST endpoints
                for type_def in type_defs.iter() {
                    let proc_metadata = metadata.get(&type_def.name);
                    let is_query = proc_metadata.map(|m| m.is_query).unwrap_or(false);

                    let path = format!("{}/{}", base_path, type_def.name);
                    let method = if is_query { "GET" } else { "POST" };

                    // Create operation
                    let mut operation = Operation {
                        summary: Some(format!("{} {}", method, type_def.name)),
                        description: None,
                        operation_id: Some(type_def.name.clone()),
                        tags: Some(vec!["RPC".to_string()]),
                        parameters: None,
                        request_body: None,
                        responses: HashMap::new(),
                    };

                    // Add response
                    let response_schema =
                        crate::openapi::OpenApiSpec::ts_to_schema(&type_def.ts_output);
                    let mut content = HashMap::new();
                    content.insert(
                        "application/json".to_string(),
                        MediaType {
                            schema: response_schema,
                            example: None,
                        },
                    );

                    operation.responses.insert(
                        "200".to_string(),
                        Response {
                            description: "Successful response".to_string(),
                            content: Some(content),
                        },
                    );

                    // Add request body for POST or query params for GET
                    if is_query {
                        // GET: Add query parameters if input type is object
                        if type_def.ts_input != "{}" {
                            let _param_schema =
                                crate::openapi::OpenApiSpec::ts_to_schema(&type_def.ts_input);
                            let mut parameters = vec![];

                            // Simple parameter extraction (could be enhanced)
                            if type_def.ts_input.contains("id") {
                                parameters.push(Parameter {
                                    name: "id".to_string(),
                                    location: ParameterLocation::Query,
                                    description: Some("Request parameter".to_string()),
                                    required: Some(true),
                                    schema: crate::openapi::Schema {
                                        schema_type: Some("string".to_string()),
                                        format: None,
                                        properties: None,
                                        required: None,
                                        items: None,
                                        reference: None,
                                    },
                                });
                            }

                            if !parameters.is_empty() {
                                operation.parameters = Some(parameters);
                            }
                        }
                    } else {
                        // POST: Add request body
                        let request_schema =
                            crate::openapi::OpenApiSpec::ts_to_schema(&type_def.ts_input);
                        let mut content = HashMap::new();
                        content.insert(
                            "application/json".to_string(),
                            MediaType {
                                schema: request_schema,
                                example: None,
                            },
                        );

                        operation.request_body = Some(RequestBody {
                            description: Some("Request body".to_string()),
                            content,
                            required: Some(true),
                        });
                    }

                    // Add to spec
                    let mut path_item = PathItem {
                        get: None,
                        post: None,
                        put: None,
                        delete: None,
                        patch: None,
                    };

                    if is_query {
                        path_item.get = Some(operation);
                    } else {
                        path_item.post = Some(operation);
                    }

                    spec.add_path(path, path_item);
                }
            }
            RpcMode::JsonRpc => {
                // Generate single JSON-RPC endpoint
                let path = "/rpc".to_string();

                let mut responses = HashMap::new();
                responses.insert(
                    "200".to_string(),
                    Response {
                        description: "Successful RPC response".to_string(),
                        content: Some({
                            let mut content = HashMap::new();
                            content.insert(
                                "application/json".to_string(),
                                MediaType {
                                    schema: crate::openapi::Schema {
                                        schema_type: Some("object".to_string()),
                                        format: None,
                                        properties: Some({
                                            let mut props = HashMap::new();
                                            props.insert(
                                                "result".to_string(),
                                                Box::new(crate::openapi::Schema {
                                                    schema_type: Some("object".to_string()),
                                                    format: None,
                                                    properties: None,
                                                    required: None,
                                                    items: None,
                                                    reference: None,
                                                }),
                                            );
                                            props
                                        }),
                                        required: Some(vec!["result".to_string()]),
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

                let operation = Operation {
                    summary: Some("JSON-RPC endpoint".to_string()),
                    description: Some(format!(
                        "Available methods: {}",
                        type_defs
                            .iter()
                            .map(|t| &t.name)
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                    operation_id: Some("rpc".to_string()),
                    tags: Some(vec!["JSON-RPC".to_string()]),
                    parameters: None,
                    request_body: Some(RequestBody {
                        description: Some("JSON-RPC request".to_string()),
                        content: {
                            let mut content = HashMap::new();
                            content.insert(
                                "application/json".to_string(),
                                MediaType {
                                    schema: crate::openapi::Schema {
                                        schema_type: Some("object".to_string()),
                                        format: None,
                                        properties: Some({
                                            let mut props = HashMap::new();
                                            props.insert(
                                                "method".to_string(),
                                                Box::new(crate::openapi::Schema {
                                                    schema_type: Some("string".to_string()),
                                                    format: None,
                                                    properties: None,
                                                    required: None,
                                                    items: None,
                                                    reference: None,
                                                }),
                                            );
                                            props.insert(
                                                "params".to_string(),
                                                Box::new(crate::openapi::Schema {
                                                    schema_type: Some("object".to_string()),
                                                    format: None,
                                                    properties: None,
                                                    required: None,
                                                    items: None,
                                                    reference: None,
                                                }),
                                            );
                                            props
                                        }),
                                        required: Some(vec!["method".to_string(), "params".to_string()]),
                                        items: None,
                                        reference: None,
                                    },
                                    example: Some(serde_json::json!({
                                        "method": type_defs.first().map(|t| &t.name).unwrap_or(&"exampleMethod".to_string()),
                                        "params": {}
                                    })),
                                },
                            );
                            content
                        },
                        required: Some(true),
                    }),
                    responses,
                };

                let path_item = PathItem {
                    get: None,
                    post: Some(operation),
                    put: None,
                    delete: None,
                    patch: None,
                };

                spec.add_path(path, path_item);
            }
        }

        spec
    }
}

impl Default for RpcRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// RPC request format
#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub method: String,
    pub params: serde_json::Value,
}

/// RPC response format
#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub result: serde_json::Value,
}

/// RPC error response format
#[derive(Debug, Serialize)]
pub struct RpcErrorResponse {
    pub error: String,
    pub code: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct TestInput {
        value: i32,
    }

    #[derive(Serialize)]
    struct TestOutput {
        result: i32,
    }

    #[tokio::test]
    async fn test_rpc_registry_creation() {
        let registry = RpcRegistry::new();
        assert_eq!(registry.list_procedures().len(), 0);
    }

    #[tokio::test]
    async fn test_rpc_registry_rest_mode() {
        let registry = RpcRegistry::new_with_mode(RpcMode::Rest);
        assert_eq!(registry.list_procedures().len(), 0);
    }

    #[tokio::test]
    async fn test_rpc_registry_jsonrpc_mode() {
        let registry = RpcRegistry::new_with_mode(RpcMode::JsonRpc);
        assert_eq!(registry.list_procedures().len(), 0);
    }

    #[tokio::test]
    async fn test_rpc_query_registration() {
        let registry = RpcRegistry::new();
        registry.query(
            "test_query",
            |input: TestInput| async move {
                Ok(TestOutput {
                    result: input.value * 2,
                })
            },
            "{ value: number }".to_string(),
            "{ result: number }".to_string(),
        );

        let procedures = registry.list_procedures();
        assert_eq!(procedures.len(), 1);
        assert_eq!(procedures[0], "test_query");
    }

    #[tokio::test]
    async fn test_rpc_mutation_registration() {
        let registry = RpcRegistry::new();
        registry.mutation(
            "test_mutation",
            |input: TestInput| async move {
                Ok(TestOutput {
                    result: input.value + 10,
                })
            },
            "{ value: number }".to_string(),
            "{ result: number }".to_string(),
        );

        let procedures = registry.list_procedures();
        assert_eq!(procedures.len(), 1);
        assert_eq!(procedures[0], "test_mutation");
    }

    #[tokio::test]
    async fn test_rpc_multiple_procedures() {
        let registry = RpcRegistry::new();

        registry.query(
            "query1",
            |_: TestInput| async move { Ok(TestOutput { result: 1 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        registry.query(
            "query2",
            |_: TestInput| async move { Ok(TestOutput { result: 2 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        registry.mutation(
            "mutation1",
            |_: TestInput| async move { Ok(TestOutput { result: 3 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        let procedures = registry.list_procedures();
        assert_eq!(procedures.len(), 3);
        assert!(procedures.contains(&"query1".to_string()));
        assert!(procedures.contains(&"query2".to_string()));
        assert!(procedures.contains(&"mutation1".to_string()));
    }

    #[tokio::test]
    async fn test_typescript_client_generation() {
        let registry = RpcRegistry::new();
        let client = registry.generate_typescript_client();
        assert!(client.contains("UltimoRpcClient"));
        assert!(client.contains("async call"));
    }

    #[tokio::test]
    async fn test_typescript_client_with_procedures() {
        let registry = RpcRegistry::new();

        registry.query(
            "getUser",
            |_: TestInput| async move { Ok(TestOutput { result: 42 }) },
            "{ id: number }".to_string(),
            "{ id: number; name: string }".to_string(),
        );

        let client = registry.generate_typescript_client();
        assert!(client.contains("UltimoRpcClient"));
        assert!(client.contains("getUser"));
        assert!(client.contains("{ id: number }"));
        assert!(client.contains("{ id: number; name: string }"));
    }

    #[tokio::test]
    async fn test_openapi_generation_rest_mode() {
        let registry = RpcRegistry::new_with_mode(RpcMode::Rest);

        registry.query(
            "testQuery",
            |_: TestInput| async move { Ok(TestOutput { result: 123 }) },
            "{ value: number }".to_string(),
            "{ result: number }".to_string(),
        );

        let openapi = registry.generate_openapi("Test API", "1.0.0", "/api");

        assert_eq!(openapi.info.title, "Test API");
        assert_eq!(openapi.info.version, "1.0.0");
        assert!(openapi.paths.contains_key("/api/testQuery"));
    }

    #[tokio::test]
    async fn test_openapi_generation_jsonrpc_mode() {
        let registry = RpcRegistry::new_with_mode(RpcMode::JsonRpc);

        registry.query(
            "testQuery",
            |_: TestInput| async move { Ok(TestOutput { result: 456 }) },
            "{ value: number }".to_string(),
            "{ result: number }".to_string(),
        );

        let openapi = registry.generate_openapi("JSON-RPC API", "2.0.0", "/rpc");

        assert_eq!(openapi.info.title, "JSON-RPC API");
        assert_eq!(openapi.info.version, "2.0.0");
        assert!(openapi.paths.contains_key("/rpc"));
    }

    #[test]
    fn test_rpc_mode_equality() {
        assert_eq!(RpcMode::Rest, RpcMode::Rest);
        assert_eq!(RpcMode::JsonRpc, RpcMode::JsonRpc);
        assert_ne!(RpcMode::Rest, RpcMode::JsonRpc);
    }

    #[tokio::test]
    async fn test_rpc_error_response() {
        let error = RpcErrorResponse {
            error: "Not Found".to_string(),
            code: 404,
        };

        assert_eq!(error.error, "Not Found");
        assert_eq!(error.code, 404);
    }

    #[tokio::test]
    async fn test_openapi_includes_all_procedures() {
        let registry = RpcRegistry::new_with_mode(RpcMode::Rest);

        registry.query(
            "getUser",
            |_: TestInput| async move { Ok(TestOutput { result: 1 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        registry.query(
            "listUsers",
            |_: TestInput| async move { Ok(TestOutput { result: 2 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        registry.mutation(
            "createUser",
            |_: TestInput| async move { Ok(TestOutput { result: 3 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        let openapi = registry.generate_openapi("API", "1.0.0", "/api");

        // In REST mode, each procedure gets its own path
        assert!(openapi.paths.contains_key("/api/getUser"));
        assert!(openapi.paths.contains_key("/api/listUsers"));
        assert!(openapi.paths.contains_key("/api/createUser"));
    }

    #[tokio::test]
    async fn test_list_procedures_returns_names() {
        let registry = RpcRegistry::new();

        registry.query(
            "proc1",
            |_: TestInput| async move { Ok(TestOutput { result: 1 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        registry.query(
            "proc2",
            |_: TestInput| async move { Ok(TestOutput { result: 2 }) },
            "{}".to_string(),
            "{}".to_string(),
        );

        let names = registry.list_procedures();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"proc1".to_string()));
        assert!(names.contains(&"proc2".to_string()));
    }
}
