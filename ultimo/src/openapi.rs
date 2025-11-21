//! OpenAPI specification generation for Ultimo
//!
//! Automatically generates OpenAPI 3.0 specs from Ultimo routes and RPC procedures.
//!
//! # Features
//!
//! - Generate OpenAPI specs from REST routes
//! - Generate OpenAPI specs from RPC registry
//! - Serve Swagger UI for interactive documentation
//! - Export specs for use with Prism, OpenAPI Generator, etc.
//!
//! # Example
//!
//! ```rust,ignore
//! use ultimo::prelude::*;
//! use ultimo::openapi::OpenApiBuilder;
//!
//! #[tokio::main]
//! async fn main() -> ultimo::Result<()> {
//!     let mut app = Ultimo::new();
//!     
//!     // Create OpenAPI configuration
//!     let mut openapi = OpenApiBuilder::new()
//!         .title("My API")
//!         .version("1.0.0")
//!         .description("User management API")
//!         .build();
//!     
//!     // Add routes (automatically tracked for OpenAPI)
//!     app.get("/users", get_users);
//!     app.post("/users", create_user);
//!     
//!     // Or use RPC registry (auto-generates OpenAPI)
//!     let rpc = RpcRegistry::new_with_mode(RpcMode::Rest);
//!     rpc.query("getUser", handler, "{ id: number }", "User");
//!     
//!     // Generate and serve OpenAPI spec
//!     openapi.register_routes(&app);
//!     openapi.register_rpc(&rpc);
//!     
//!     app.get("/openapi.json", openapi.spec_handler());
//!     app.get("/docs", openapi.swagger_ui_handler());
//!     
//!     app.listen("127.0.0.1:3000").await
//! }
//! ```

pub mod docs;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OpenAPI 3.0 Specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String, // "3.0.0"
    pub info: Info,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<Server>>,
    pub paths: HashMap<String, PathItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    pub responses: HashMap<String, Response>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: ParameterLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    pub schema: Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Query,
    Path,
    Header,
    Cookie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: HashMap<String, MediaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, MediaType>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    pub schema: Schema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Box<Schema>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Schema>>,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<HashMap<String, Schema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Builder for OpenAPI specifications
pub struct OpenApiBuilder {
    title: String,
    version: String,
    description: Option<String>,
    servers: Vec<Server>,
    tags: Vec<Tag>,
    contact: Option<Contact>,
    license: Option<License>,
}

impl OpenApiBuilder {
    /// Create a new OpenAPI builder
    pub fn new() -> Self {
        Self {
            title: "API".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            servers: vec![],
            tags: vec![],
            contact: None,
            license: None,
        }
    }

    /// Set API title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set API version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set API description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a server
    pub fn server(mut self, url: impl Into<String>, description: Option<String>) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description,
        });
        self
    }

    /// Add a tag
    pub fn tag(mut self, name: impl Into<String>, description: Option<String>) -> Self {
        self.tags.push(Tag {
            name: name.into(),
            description,
        });
        self
    }

    /// Set contact information
    pub fn contact(
        mut self,
        name: Option<String>,
        email: Option<String>,
        url: Option<String>,
    ) -> Self {
        self.contact = Some(Contact { name, email, url });
        self
    }

    /// Set license
    pub fn license(mut self, name: impl Into<String>, url: Option<String>) -> Self {
        self.license = Some(License {
            name: name.into(),
            url,
        });
        self
    }

    /// Build the OpenAPI spec (without paths yet)
    pub fn build(self) -> OpenApiSpec {
        OpenApiSpec {
            openapi: "3.0.0".to_string(),
            info: Info {
                title: self.title,
                version: self.version,
                description: self.description,
                contact: self.contact,
                license: self.license,
            },
            servers: if self.servers.is_empty() {
                None
            } else {
                Some(self.servers)
            },
            paths: HashMap::new(),
            components: None,
            tags: if self.tags.is_empty() {
                None
            } else {
                Some(self.tags)
            },
        }
    }
}

impl Default for OpenApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenApiSpec {
    /// Add a path to the spec
    pub fn add_path(&mut self, path: String, item: PathItem) {
        self.paths.insert(path, item);
    }

    /// Add a schema to components
    pub fn add_schema(&mut self, name: String, schema: Schema) {
        if self.components.is_none() {
            self.components = Some(Components {
                schemas: Some(HashMap::new()),
            });
        }

        if let Some(components) = &mut self.components {
            if components.schemas.is_none() {
                components.schemas = Some(HashMap::new());
            }
            if let Some(schemas) = &mut components.schemas {
                schemas.insert(name, schema);
            }
        }
    }

    /// Convert TypeScript type string to OpenAPI schema
    pub fn ts_to_schema(ts_type: &str) -> Schema {
        // Simple type mapping - can be expanded
        match ts_type {
            "string" => Schema {
                schema_type: Some("string".to_string()),
                format: None,
                properties: None,
                required: None,
                items: None,
                reference: None,
            },
            "number" => Schema {
                schema_type: Some("number".to_string()),
                format: None,
                properties: None,
                required: None,
                items: None,
                reference: None,
            },
            "boolean" => Schema {
                schema_type: Some("boolean".to_string()),
                format: None,
                properties: None,
                required: None,
                items: None,
                reference: None,
            },
            _ if ts_type.ends_with("[]") => {
                let item_type = &ts_type[..ts_type.len() - 2];
                Schema {
                    schema_type: Some("array".to_string()),
                    format: None,
                    properties: None,
                    required: None,
                    items: Some(Box::new(Self::ts_to_schema(item_type))),
                    reference: None,
                }
            }
            _ => Schema {
                schema_type: Some("object".to_string()),
                format: None,
                properties: None,
                required: None,
                items: None,
                reference: None,
            },
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write to file
    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = self
            .to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Get Swagger UI HTML with this spec's URL
    pub fn swagger_ui_html(&self, spec_url: &str) -> String {
        docs::SWAGGER_UI_HTML.replace("{OPENAPI_URL}", spec_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder() {
        let spec = OpenApiBuilder::new()
            .title("Test API")
            .version("1.0.0")
            .description("A test API")
            .server(
                "http://localhost:3000",
                Some("Development server".to_string()),
            )
            .tag("users", Some("User operations".to_string()))
            .build();

        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");
        assert!(spec.servers.is_some());
        assert!(spec.tags.is_some());
    }

    #[test]
    fn test_builder_minimal() {
        let spec = OpenApiBuilder::new()
            .title("Minimal API")
            .version("0.1.0")
            .build();

        assert_eq!(spec.info.title, "Minimal API");
        assert_eq!(spec.info.version, "0.1.0");
        assert_eq!(spec.openapi, "3.0.0");
        assert!(spec.paths.is_empty());
    }

    #[test]
    fn test_builder_with_contact() {
        let spec = OpenApiBuilder::new()
            .title("Contact API")
            .version("1.0.0")
            .contact(
                Some("Support".to_string()),
                Some("support@example.com".to_string()),
                Some("https://example.com".to_string()),
            )
            .build();

        assert!(spec.info.contact.is_some());
        let contact = spec.info.contact.unwrap();
        assert_eq!(contact.name, Some("Support".to_string()));
        assert_eq!(contact.email, Some("support@example.com".to_string()));
    }

    #[test]
    fn test_builder_with_license() {
        let spec = OpenApiBuilder::new()
            .title("Licensed API")
            .version("1.0.0")
            .license(
                "MIT".to_string(),
                Some("https://opensource.org/licenses/MIT".to_string()),
            )
            .build();

        assert!(spec.info.license.is_some());
        let license = spec.info.license.unwrap();
        assert_eq!(license.name, "MIT");
        assert_eq!(
            license.url,
            Some("https://opensource.org/licenses/MIT".to_string())
        );
    }

    #[test]
    fn test_builder_multiple_servers() {
        let spec = OpenApiBuilder::new()
            .title("Multi-server API")
            .version("1.0.0")
            .server("http://localhost:3000", Some("Dev".to_string()))
            .server("https://api.example.com", Some("Production".to_string()))
            .build();

        assert!(spec.servers.is_some());
        let servers = spec.servers.unwrap();
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].url, "http://localhost:3000");
        assert_eq!(servers[1].url, "https://api.example.com");
    }

    #[test]
    fn test_builder_multiple_tags() {
        let spec = OpenApiBuilder::new()
            .title("Tagged API")
            .version("1.0.0")
            .tag("users", Some("User management".to_string()))
            .tag("posts", Some("Post management".to_string()))
            .tag("admin", None)
            .build();

        assert!(spec.tags.is_some());
        let tags = spec.tags.unwrap();
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].name, "users");
        assert_eq!(tags[1].name, "posts");
        assert_eq!(tags[2].name, "admin");
    }

    #[test]
    fn test_ts_to_schema() {
        let schema = OpenApiSpec::ts_to_schema("string");
        assert_eq!(schema.schema_type, Some("string".to_string()));

        let array_schema = OpenApiSpec::ts_to_schema("string[]");
        assert_eq!(array_schema.schema_type, Some("array".to_string()));
        assert!(array_schema.items.is_some());
    }

    #[test]
    fn test_ts_to_schema_number() {
        let schema = OpenApiSpec::ts_to_schema("number");
        assert_eq!(schema.schema_type, Some("number".to_string()));
    }

    #[test]
    fn test_ts_to_schema_boolean() {
        let schema = OpenApiSpec::ts_to_schema("boolean");
        assert_eq!(schema.schema_type, Some("boolean".to_string()));
    }

    #[test]
    fn test_ts_to_schema_object() {
        let schema = OpenApiSpec::ts_to_schema("object");
        assert_eq!(schema.schema_type, Some("object".to_string()));
    }

    #[test]
    fn test_ts_to_schema_number_array() {
        let schema = OpenApiSpec::ts_to_schema("number[]");
        assert_eq!(schema.schema_type, Some("array".to_string()));
        assert!(schema.items.is_some());

        if let Some(items) = schema.items {
            assert_eq!(items.schema_type, Some("number".to_string()));
        }
    }

    #[test]
    fn test_ts_to_schema_unknown() {
        let schema = OpenApiSpec::ts_to_schema("UnknownType");
        assert_eq!(schema.schema_type, Some("object".to_string()));
    }

    #[test]
    fn test_openapi_spec_version() {
        let spec = OpenApiBuilder::new()
            .title("Version Test")
            .version("1.0.0")
            .build();

        assert_eq!(spec.openapi, "3.0.0");
    }

    #[test]
    fn test_path_item_creation() {
        let path = PathItem {
            get: None,
            post: None,
            put: None,
            delete: None,
            patch: None,
        };

        assert!(path.get.is_none());
        assert!(path.post.is_none());
    }

    #[test]
    fn test_operation_creation() {
        let operation = Operation {
            summary: Some("Get user".to_string()),
            description: Some("Retrieve a user by ID".to_string()),
            operation_id: Some("getUser".to_string()),
            tags: Some(vec!["users".to_string()]),
            parameters: None,
            request_body: None,
            responses: std::collections::HashMap::new(),
        };

        assert_eq!(operation.summary, Some("Get user".to_string()));
        assert_eq!(operation.operation_id, Some("getUser".to_string()));
    }

    #[test]
    fn test_schema_with_properties() {
        let mut properties = std::collections::HashMap::new();
        properties.insert(
            "id".to_string(),
            Box::new(Schema {
                schema_type: Some("number".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                reference: None,
            }),
        );

        let schema = Schema {
            schema_type: Some("object".to_string()),
            format: None,
            items: None,
            properties: Some(properties),
            required: Some(vec!["id".to_string()]),
            reference: None,
        };

        assert_eq!(schema.schema_type, Some("object".to_string()));
        assert!(schema.properties.is_some());
        assert_eq!(schema.required, Some(vec!["id".to_string()]));
    }

    #[test]
    fn test_parameter_creation() {
        let param = Parameter {
            name: "userId".to_string(),
            location: ParameterLocation::Path,
            description: Some("User ID".to_string()),
            required: Some(true),
            schema: Schema {
                schema_type: Some("string".to_string()),
                format: None,
                items: None,
                properties: None,
                required: None,
                reference: None,
            },
        };

        assert_eq!(param.name, "userId");
        assert_eq!(param.required, Some(true));
    }

    #[test]
    fn test_info_creation() {
        let info = Info {
            title: "Test API".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Description".to_string()),
            contact: None,
            license: None,
        };

        assert_eq!(info.title, "Test API");
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn test_server_creation() {
        let server = Server {
            url: "https://api.example.com".to_string(),
            description: Some("Production".to_string()),
        };

        assert_eq!(server.url, "https://api.example.com");
        assert_eq!(server.description, Some("Production".to_string()));
    }

    #[test]
    fn test_tag_creation() {
        let tag = Tag {
            name: "users".to_string(),
            description: Some("User operations".to_string()),
        };

        assert_eq!(tag.name, "users");
        assert_eq!(tag.description, Some("User operations".to_string()));
    }

    #[test]
    fn test_json_serialization() {
        let spec = OpenApiBuilder::new()
            .title("JSON Test")
            .version("1.0.0")
            .build();

        let json = serde_json::to_string(&spec);
        assert!(json.is_ok());

        let json_value = json.unwrap();
        assert!(json_value.contains("JSON Test"));
        assert!(json_value.contains("1.0.0"));
        assert!(json_value.contains("3.0.0"));
    }

    #[test]
    fn test_swagger_ui_html_generation() {
        let spec = OpenApiBuilder::new()
            .title("Swagger Test")
            .version("1.0.0")
            .build();

        let html = spec.swagger_ui_html("/openapi.json");
        assert!(html.contains("Swagger UI"));
        assert!(html.contains("/openapi.json"));
    }
}
