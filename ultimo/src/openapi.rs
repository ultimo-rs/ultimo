//! OpenAPI specification generation for Ultimo
//!
//! Build an OpenAPI 3.0 spec for your REST routes, or generate one from an
//! [`RpcRegistry`](crate::RpcRegistry), and serve it with Swagger UI.
//!
//! # Features
//!
//! - Describe REST routes with [`Operation`] and [`PathItem`]
//! - Generate a spec from an RPC registry ([`RpcRegistry::generate_openapi`](crate::RpcRegistry::generate_openapi))
//! - Document authentication with [`SecurityScheme`]s, so Swagger UI shows an
//!   **Authorize** button and sends credentials with "Try it out"
//! - Serve Swagger UI for interactive documentation
//! - Export specs for use with Prism, OpenAPI Generator, etc.
//!
//! # Example
//!
//! ```rust
//! use ultimo::openapi::{OpenApiBuilder, Operation, PathItem, Response, Schema, SecurityScheme};
//!
//! let mut spec = OpenApiBuilder::new()
//!     .title("My API")
//!     .version("1.0.0")
//!     // Declare how clients authenticate...
//!     .security_scheme("bearerAuth", SecurityScheme::bearer_jwt())
//!     // ...and require it on every operation by default.
//!     .security("bearerAuth")
//!     .build();
//!
//! spec.add_path(
//!     "/users",
//!     PathItem::new()
//!         .get(
//!             Operation::new()
//!                 .summary("List users")
//!                 .response("200", Response::json("Users", Schema::array(Schema::reference("User")))),
//!         )
//!         .post(Operation::new().summary("Create a user").response("201", Response::new("Created"))),
//! );
//!
//! // A public endpoint opts out of the default requirement.
//! spec.add_path(
//!     "/health",
//!     PathItem::new().get(Operation::new().public().response("200", Response::new("OK"))),
//! );
//!
//! let json = spec.to_json().unwrap();
//! assert!(json.contains("securitySchemes"));
//! ```
//!
//! # Constructing the types
//!
//! The model types are `#[non_exhaustive]` so new OpenAPI fields can be added
//! without breaking your code. Build them with their constructors and chainable
//! setters (`Operation::new().summary(..)`), or set any public field directly
//! on a value you already have (`op.description = Some(..)`).

pub mod docs;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// One way of satisfying authentication: scheme name → required scopes.
///
/// Every scheme in one requirement must be satisfied together (AND). A list
/// of requirements offers alternatives (OR). Scopes only apply to OAuth2 and
/// OpenID Connect schemes; use an empty list for the others.
pub type SecurityRequirement = BTreeMap<String, Vec<String>>;

fn requirement(scheme: impl Into<String>, scopes: &[&str]) -> SecurityRequirement {
    BTreeMap::from([(
        scheme.into(),
        scopes.iter().map(|s| s.to_string()).collect(),
    )])
}

/// OpenAPI 3.0 Specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OpenApiSpec {
    pub openapi: String, // "3.0.0"
    pub info: Info,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servers: Option<Vec<Server>>,
    pub paths: HashMap<String, PathItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    /// Security applied to every operation that doesn't set its own.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<SecurityRequirement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
}

/// General information about the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
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

impl Info {
    /// Info with a title and version.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            version: version.into(),
            description: None,
            contact: None,
            license: None,
        }
    }
}

/// Contact information for the API.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// License information for the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct License {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl License {
    /// A license by name, e.g. `"MIT"`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: None,
        }
    }
}

/// A server the API is available on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Server {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Server {
    /// A server at `url`.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            description: None,
        }
    }
}

/// The operations available on one path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
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

impl PathItem {
    /// An empty path item; add operations with [`get`](Self::get), [`post`](Self::post), ….
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `GET` operation.
    pub fn get(mut self, op: Operation) -> Self {
        self.get = Some(op);
        self
    }

    /// Set the `POST` operation.
    pub fn post(mut self, op: Operation) -> Self {
        self.post = Some(op);
        self
    }

    /// Set the `PUT` operation.
    pub fn put(mut self, op: Operation) -> Self {
        self.put = Some(op);
        self
    }

    /// Set the `DELETE` operation.
    pub fn delete(mut self, op: Operation) -> Self {
        self.delete = Some(op);
        self
    }

    /// Set the `PATCH` operation.
    pub fn patch(mut self, op: Operation) -> Self {
        self.patch = Some(op);
        self
    }

    /// The operation for an HTTP method (case-insensitive), if set.
    pub fn operation_mut(&mut self, method: &str) -> Option<&mut Operation> {
        match method.to_ascii_lowercase().as_str() {
            "get" => self.get.as_mut(),
            "post" => self.post.as_mut(),
            "put" => self.put.as_mut(),
            "delete" => self.delete.as_mut(),
            "patch" => self.patch.as_mut(),
            _ => None,
        }
    }
}

/// One API operation (a method on a path).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
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
    /// Security for this operation. `None` inherits the spec-wide default;
    /// `Some(vec![])` marks the operation public.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<SecurityRequirement>>,
    /// Marks the operation as deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
}

impl Operation {
    /// An operation with no responses yet; add them with [`response`](Self::response).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the one-line summary.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Set the longer description (Markdown).
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the operation id, used as the method name by client generators.
    pub fn operation_id(mut self, id: impl Into<String>) -> Self {
        self.operation_id = Some(id.into());
        self
    }

    /// Add a tag (Swagger UI groups operations by tag).
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.get_or_insert_with(Vec::new).push(tag.into());
        self
    }

    /// Add a parameter.
    pub fn parameter(mut self, param: Parameter) -> Self {
        self.parameters.get_or_insert_with(Vec::new).push(param);
        self
    }

    /// Set the request body.
    pub fn request_body(mut self, body: RequestBody) -> Self {
        self.request_body = Some(body);
        self
    }

    /// Add a response for a status code (`"200"`, `"404"`, `"default"`, …).
    pub fn response(mut self, status: impl Into<String>, response: Response) -> Self {
        self.responses.insert(status.into(), response);
        self
    }

    /// Require a security scheme (declared with
    /// [`OpenApiBuilder::security_scheme`]) for this operation. Calling it
    /// again adds an *alternative*: any one of them is enough.
    pub fn security(self, scheme: impl Into<String>) -> Self {
        self.security_with_scopes(scheme, &[])
    }

    /// Like [`security`](Self::security), requiring OAuth2/OpenID Connect scopes.
    pub fn security_with_scopes(mut self, scheme: impl Into<String>, scopes: &[&str]) -> Self {
        self.security
            .get_or_insert_with(Vec::new)
            .push(requirement(scheme, scopes));
        self
    }

    /// Mark the operation public: no authentication, even when the spec has a
    /// default [`security`](OpenApiBuilder::security) requirement.
    pub fn public(mut self) -> Self {
        self.security = Some(Vec::new());
        self
    }

    /// Mark the operation deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = Some(true);
        self
    }
}

/// A parameter in the path, query string, headers or cookies.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
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

impl Parameter {
    /// A parameter of any location.
    pub fn new(name: impl Into<String>, location: ParameterLocation, schema: Schema) -> Self {
        Self {
            name: name.into(),
            location,
            description: None,
            required: None,
            schema,
        }
    }

    /// A path parameter (`/users/{id}`); path parameters are always required.
    pub fn path(name: impl Into<String>, schema: Schema) -> Self {
        Self::new(name, ParameterLocation::Path, schema).required(true)
    }

    /// A query-string parameter.
    pub fn query(name: impl Into<String>, schema: Schema) -> Self {
        Self::new(name, ParameterLocation::Query, schema)
    }

    /// A request header.
    pub fn header(name: impl Into<String>, schema: Schema) -> Self {
        Self::new(name, ParameterLocation::Header, schema)
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set whether the parameter is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
}

/// Where a [`Parameter`] is sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Query,
    Path,
    Header,
    Cookie,
}

/// The body of a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: HashMap<String, MediaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

impl RequestBody {
    /// A required JSON body with this schema.
    pub fn json(schema: Schema) -> Self {
        Self {
            description: None,
            content: HashMap::from([("application/json".to_string(), MediaType::new(schema))]),
            required: Some(true),
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set whether the body is required.
    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
}

/// A response to an operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Response {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, MediaType>>,
}

impl Response {
    /// A response with no body.
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            content: None,
        }
    }

    /// A response with a JSON body of this schema.
    pub fn json(description: impl Into<String>, schema: Schema) -> Self {
        Self {
            description: description.into(),
            content: Some(HashMap::from([(
                "application/json".to_string(),
                MediaType::new(schema),
            )])),
        }
    }
}

/// A body's schema and example for one content type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MediaType {
    pub schema: Schema,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

impl MediaType {
    /// A media type with this schema.
    pub fn new(schema: Schema) -> Self {
        Self {
            schema,
            example: None,
        }
    }

    /// Set an example value.
    pub fn example(mut self, example: serde_json::Value) -> Self {
        self.example = Some(example);
        self
    }
}

/// A JSON Schema (the OpenAPI 3.0 subset).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
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

impl Schema {
    /// A schema of a primitive type: `"string"`, `"integer"`, `"number"`,
    /// `"boolean"`, `"object"` or `"array"`.
    pub fn of_type(schema_type: impl Into<String>) -> Self {
        Self {
            schema_type: Some(schema_type.into()),
            ..Self::default()
        }
    }

    /// A `string` schema.
    pub fn string() -> Self {
        Self::of_type("string")
    }

    /// An `integer` schema.
    pub fn integer() -> Self {
        Self::of_type("integer")
    }

    /// An array of `items`.
    pub fn array(items: Schema) -> Self {
        Self {
            schema_type: Some("array".to_string()),
            items: Some(Box::new(items)),
            ..Self::default()
        }
    }

    /// An object with these properties, all of them required.
    pub fn object<I, K>(properties: I) -> Self
    where
        I: IntoIterator<Item = (K, Schema)>,
        K: Into<String>,
    {
        let properties: HashMap<String, Box<Schema>> = properties
            .into_iter()
            .map(|(k, v)| (k.into(), Box::new(v)))
            .collect();
        let mut required: Vec<String> = properties.keys().cloned().collect();
        required.sort();
        Self {
            schema_type: Some("object".to_string()),
            properties: Some(properties),
            required: Some(required),
            ..Self::default()
        }
    }

    /// A reference to a schema in `components.schemas`, by name (`"User"`) or
    /// full pointer (`"#/components/schemas/User"`).
    pub fn reference(name: impl Into<String>) -> Self {
        let name = name.into();
        let reference = if name.starts_with('#') {
            name
        } else {
            format!("#/components/schemas/{name}")
        };
        Self {
            reference: Some(reference),
            ..Self::default()
        }
    }

    /// Set the format (`"date-time"`, `"uuid"`, `"int64"`, …).
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
}

/// Reusable components: schemas and security schemes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Components {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schemas: Option<HashMap<String, Schema>>,
    #[serde(rename = "securitySchemes", skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<BTreeMap<String, SecurityScheme>>,
}

/// A tag for grouping operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Tag {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Tag {
    /// A tag by name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
        }
    }
}

/// How clients authenticate — an entry in `components.securitySchemes`.
///
/// Each constructor matches one of Ultimo's auth features:
///
/// | Ultimo feature                        | Scheme                                               |
/// | ------------------------------------- | ---------------------------------------------------- |
/// | `Jwt::hs256` / `Jwt::oidc` (`jwt`)    | [`bearer_jwt`](Self::bearer_jwt)                     |
/// | `ApiKey` (`api-key`)                  | [`api_key_header`](Self::api_key_header)             |
/// | `BasicAuth`                           | [`basic`](Self::basic)                               |
/// | OIDC provider tokens                  | [`open_id_connect`](Self::open_id_connect)           |
/// | OAuth2 provider tokens                | [`oauth2_authorization_code`](Self::oauth2_authorization_code) |
///
/// Declaring a scheme documents it; it doesn't enforce anything. Your
/// middleware still does the checking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
pub enum SecurityScheme {
    /// HTTP authentication (`Authorization: <scheme> <credentials>`).
    #[serde(rename = "http")]
    #[non_exhaustive]
    Http {
        /// `"bearer"` or `"basic"`.
        scheme: String,
        /// A hint for the bearer token format, e.g. `"JWT"`.
        #[serde(rename = "bearerFormat", skip_serializing_if = "Option::is_none")]
        bearer_format: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// An API key in a header, query parameter or cookie.
    #[serde(rename = "apiKey")]
    #[non_exhaustive]
    ApiKey {
        /// The header, query parameter or cookie name.
        name: String,
        /// Where the key is sent (`header`, `query` or `cookie`).
        #[serde(rename = "in")]
        location: ParameterLocation,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// OAuth2, with the flows the API supports.
    #[serde(rename = "oauth2")]
    #[non_exhaustive]
    OAuth2 {
        flows: OAuthFlows,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    /// OpenID Connect discovery.
    #[serde(rename = "openIdConnect")]
    #[non_exhaustive]
    OpenIdConnect {
        /// The provider's `/.well-known/openid-configuration` URL.
        #[serde(rename = "openIdConnectUrl")]
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
}

impl SecurityScheme {
    /// `Authorization: Bearer <token>`, with an opaque token.
    pub fn bearer() -> Self {
        SecurityScheme::Http {
            scheme: "bearer".to_string(),
            bearer_format: None,
            description: None,
        }
    }

    /// `Authorization: Bearer <JWT>` — what the `jwt` and `oidc` features verify.
    pub fn bearer_jwt() -> Self {
        SecurityScheme::Http {
            scheme: "bearer".to_string(),
            bearer_format: Some("JWT".to_string()),
            description: None,
        }
    }

    /// HTTP Basic auth — what `BasicAuth` checks.
    pub fn basic() -> Self {
        SecurityScheme::Http {
            scheme: "basic".to_string(),
            bearer_format: None,
            description: None,
        }
    }

    /// An API key in a request header, e.g. `"X-API-Key"`.
    pub fn api_key_header(name: impl Into<String>) -> Self {
        Self::api_key(name, ParameterLocation::Header)
    }

    /// An API key in a query parameter.
    pub fn api_key_query(name: impl Into<String>) -> Self {
        Self::api_key(name, ParameterLocation::Query)
    }

    /// An API key in a cookie.
    pub fn api_key_cookie(name: impl Into<String>) -> Self {
        Self::api_key(name, ParameterLocation::Cookie)
    }

    fn api_key(name: impl Into<String>, location: ParameterLocation) -> Self {
        SecurityScheme::ApiKey {
            name: name.into(),
            location,
            description: None,
        }
    }

    /// OpenID Connect, discovered from the provider's
    /// `/.well-known/openid-configuration` URL.
    pub fn open_id_connect(url: impl Into<String>) -> Self {
        SecurityScheme::OpenIdConnect {
            url: url.into(),
            description: None,
        }
    }

    /// OAuth2 authorization-code flow (with PKCE in Swagger UI), given the
    /// provider's endpoints and the scopes it offers (`name → description`).
    pub fn oauth2_authorization_code(
        authorization_url: impl Into<String>,
        token_url: impl Into<String>,
        scopes: &[(&str, &str)],
    ) -> Self {
        SecurityScheme::OAuth2 {
            flows: OAuthFlows {
                authorization_code: Some(OAuthFlow {
                    authorization_url: Some(authorization_url.into()),
                    token_url: Some(token_url.into()),
                    refresh_url: None,
                    scopes: scopes_map(scopes),
                }),
                client_credentials: None,
            },
            description: None,
        }
    }

    /// OAuth2 client-credentials flow (service-to-service).
    pub fn oauth2_client_credentials(
        token_url: impl Into<String>,
        scopes: &[(&str, &str)],
    ) -> Self {
        SecurityScheme::OAuth2 {
            flows: OAuthFlows {
                authorization_code: None,
                client_credentials: Some(OAuthFlow {
                    authorization_url: None,
                    token_url: Some(token_url.into()),
                    refresh_url: None,
                    scopes: scopes_map(scopes),
                }),
            },
            description: None,
        }
    }

    /// Set the description shown in Swagger UI's Authorize dialog.
    pub fn description(mut self, text: impl Into<String>) -> Self {
        let text = Some(text.into());
        match &mut self {
            SecurityScheme::Http { description, .. }
            | SecurityScheme::ApiKey { description, .. }
            | SecurityScheme::OAuth2 { description, .. }
            | SecurityScheme::OpenIdConnect { description, .. } => *description = text,
        }
        self
    }
}

fn scopes_map(scopes: &[(&str, &str)]) -> BTreeMap<String, String> {
    scopes
        .iter()
        .map(|(name, desc)| (name.to_string(), desc.to_string()))
        .collect()
}

/// The OAuth2 flows a [`SecurityScheme::OAuth2`] supports.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OAuthFlows {
    #[serde(rename = "authorizationCode", skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<OAuthFlow>,
    #[serde(rename = "clientCredentials", skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<OAuthFlow>,
}

/// One OAuth2 flow's endpoints and scopes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OAuthFlow {
    #[serde(rename = "authorizationUrl", skip_serializing_if = "Option::is_none")]
    pub authorization_url: Option<String>,
    #[serde(rename = "tokenUrl", skip_serializing_if = "Option::is_none")]
    pub token_url: Option<String>,
    #[serde(rename = "refreshUrl", skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    /// Scope name → description. Required by OpenAPI, even if empty.
    pub scopes: BTreeMap<String, String>,
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
    security_schemes: BTreeMap<String, SecurityScheme>,
    security: Vec<SecurityRequirement>,
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
            security_schemes: BTreeMap::new(),
            security: vec![],
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

    /// Declare a security scheme under a name, which operations then refer
    /// to. Swagger UI shows an **Authorize** button for declared schemes.
    pub fn security_scheme(mut self, name: impl Into<String>, scheme: SecurityScheme) -> Self {
        self.security_schemes.insert(name.into(), scheme);
        self
    }

    /// Require a declared scheme on every operation by default. Operations
    /// can override it with [`Operation::security`] or opt out with
    /// [`Operation::public`]. Calling it again adds an alternative: any one
    /// requirement is enough.
    pub fn security(self, scheme: impl Into<String>) -> Self {
        self.security_with_scopes(scheme, &[])
    }

    /// Like [`security`](Self::security), requiring OAuth2/OpenID Connect scopes.
    pub fn security_with_scopes(mut self, scheme: impl Into<String>, scopes: &[&str]) -> Self {
        self.security.push(requirement(scheme, scopes));
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
            components: if self.security_schemes.is_empty() {
                None
            } else {
                Some(Components {
                    schemas: None,
                    security_schemes: Some(self.security_schemes),
                })
            },
            security: if self.security.is_empty() {
                None
            } else {
                Some(self.security)
            },
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
    /// Add (or replace) a path.
    pub fn add_path(&mut self, path: impl Into<String>, item: PathItem) {
        self.paths.insert(path.into(), item);
    }

    /// Add a schema to components
    pub fn add_schema(&mut self, name: impl Into<String>, schema: Schema) {
        self.components
            .get_or_insert_with(Components::default)
            .schemas
            .get_or_insert_with(HashMap::new)
            .insert(name.into(), schema);
    }

    /// Declare a security scheme on an existing spec — for example one from
    /// [`RpcRegistry::generate_openapi`](crate::RpcRegistry::generate_openapi).
    pub fn add_security_scheme(&mut self, name: impl Into<String>, scheme: SecurityScheme) {
        self.components
            .get_or_insert_with(Components::default)
            .security_schemes
            .get_or_insert_with(BTreeMap::new)
            .insert(name.into(), scheme);
    }

    /// Require a declared scheme on every operation by default (see
    /// [`OpenApiBuilder::security`]).
    pub fn require_security(&mut self, scheme: impl Into<String>) {
        self.security
            .get_or_insert_with(Vec::new)
            .push(requirement(scheme, &[]));
    }

    /// The operation for `method` (case-insensitive) on `path`, to adjust it
    /// after the spec is built — e.g. to mark one generated RPC endpoint
    /// public with `op.security = Some(vec![])`.
    pub fn operation_mut(&mut self, path: &str, method: &str) -> Option<&mut Operation> {
        self.paths.get_mut(path)?.operation_mut(method)
    }

    /// Convert TypeScript type string to OpenAPI schema
    pub fn ts_to_schema(ts_type: &str) -> Schema {
        // Simple type mapping - can be expanded
        match ts_type {
            "string" | "number" | "boolean" => Schema::of_type(ts_type),
            _ if ts_type.ends_with("[]") => {
                Schema::array(Self::ts_to_schema(&ts_type[..ts_type.len() - 2]))
            }
            _ => Schema::of_type("object"),
        }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write to file
    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = self.to_json().map_err(std::io::Error::other)?;
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
        let operation = Operation::new()
            .summary("Get user")
            .description("Retrieve a user by ID")
            .operation_id("getUser")
            .tag("users");

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

    fn json(spec: &OpenApiSpec) -> serde_json::Value {
        serde_json::from_str(&spec.to_json().unwrap()).unwrap()
    }

    #[test]
    fn security_schemes_serialize_per_openapi() {
        let spec = OpenApiBuilder::new()
            .security_scheme("bearerAuth", SecurityScheme::bearer_jwt())
            .security_scheme("basicAuth", SecurityScheme::basic())
            .security_scheme(
                "apiKey",
                SecurityScheme::api_key_header("X-API-Key").description("Issued in the dashboard"),
            )
            .security_scheme(
                "oidc",
                SecurityScheme::open_id_connect(
                    "https://accounts.google.com/.well-known/openid-configuration",
                ),
            )
            .security_scheme(
                "oauth",
                SecurityScheme::oauth2_authorization_code(
                    "https://auth.example.com/authorize",
                    "https://auth.example.com/token",
                    &[("read:users", "Read users")],
                ),
            )
            .security_scheme(
                "service",
                SecurityScheme::oauth2_client_credentials("https://auth.example.com/token", &[]),
            )
            .build();

        let v = json(&spec);
        let schemes = &v["components"]["securitySchemes"];
        assert_eq!(
            schemes["bearerAuth"],
            serde_json::json!({"type": "http", "scheme": "bearer", "bearerFormat": "JWT"})
        );
        assert_eq!(
            schemes["basicAuth"],
            serde_json::json!({"type": "http", "scheme": "basic"})
        );
        assert_eq!(
            schemes["apiKey"],
            serde_json::json!({
                "type": "apiKey", "name": "X-API-Key", "in": "header",
                "description": "Issued in the dashboard"
            })
        );
        assert_eq!(
            schemes["oidc"],
            serde_json::json!({
                "type": "openIdConnect",
                "openIdConnectUrl": "https://accounts.google.com/.well-known/openid-configuration"
            })
        );
        assert_eq!(
            schemes["oauth"],
            serde_json::json!({
                "type": "oauth2",
                "flows": {"authorizationCode": {
                    "authorizationUrl": "https://auth.example.com/authorize",
                    "tokenUrl": "https://auth.example.com/token",
                    "scopes": {"read:users": "Read users"}
                }}
            })
        );
        // `scopes` is required by OpenAPI even when empty.
        assert_eq!(
            schemes["service"]["flows"]["clientCredentials"]["scopes"],
            serde_json::json!({})
        );
        // No global requirement unless asked for.
        assert!(v.get("security").is_none());
    }

    #[test]
    fn security_schemes_round_trip_through_serde() {
        let scheme = SecurityScheme::api_key_query("token");
        let text = serde_json::to_string(&scheme).unwrap();
        let back: SecurityScheme = serde_json::from_str(&text).unwrap();
        assert_eq!(back, scheme);
    }

    #[test]
    fn global_and_per_operation_requirements() {
        let mut spec = OpenApiBuilder::new()
            .security_scheme("bearerAuth", SecurityScheme::bearer_jwt())
            .security_scheme(
                "oauth",
                SecurityScheme::open_id_connect(
                    "https://id.example/.well-known/openid-configuration",
                ),
            )
            .security("bearerAuth")
            .build();
        spec.add_path(
            "/users",
            PathItem::new()
                .get(Operation::new().response("200", Response::new("OK")))
                .delete(
                    Operation::new()
                        .security_with_scopes("oauth", &["admin"])
                        .security("bearerAuth")
                        .deprecated()
                        .response("204", Response::new("Deleted")),
                ),
        );
        spec.add_path("/health", PathItem::new().get(Operation::new().public()));

        let v = json(&spec);
        assert_eq!(v["security"], serde_json::json!([{"bearerAuth": []}]));
        // Inherits the default: no `security` key on the operation.
        assert!(v["paths"]["/users"]["get"].get("security").is_none());
        // Alternatives, in order, with scopes.
        assert_eq!(
            v["paths"]["/users"]["delete"]["security"],
            serde_json::json!([{"oauth": ["admin"]}, {"bearerAuth": []}])
        );
        assert_eq!(v["paths"]["/users"]["delete"]["deprecated"], true);
        // Public: an explicit empty list overrides the default.
        assert_eq!(
            v["paths"]["/health"]["get"]["security"],
            serde_json::json!([])
        );
    }

    #[test]
    fn securing_a_generated_spec_after_the_fact() {
        let mut spec = OpenApiBuilder::new().build();
        spec.add_path("/rpc", PathItem::new().post(Operation::new()));
        spec.add_path("/health", PathItem::new().get(Operation::new()));

        spec.add_security_scheme("apiKey", SecurityScheme::api_key_header("X-API-Key"));
        spec.require_security("apiKey");
        spec.operation_mut("/health", "GET").unwrap().security = Some(vec![]);
        assert!(spec.operation_mut("/health", "post").is_none());
        assert!(spec.operation_mut("/missing", "get").is_none());

        let v = json(&spec);
        assert_eq!(v["components"]["securitySchemes"]["apiKey"]["in"], "header");
        assert_eq!(v["security"], serde_json::json!([{"apiKey": []}]));
        assert_eq!(
            v["paths"]["/health"]["get"]["security"],
            serde_json::json!([])
        );
    }

    #[test]
    fn schema_helpers() {
        assert_eq!(
            Schema::reference("User").reference.as_deref(),
            Some("#/components/schemas/User")
        );
        assert_eq!(
            Schema::reference("#/components/schemas/Other")
                .reference
                .as_deref(),
            Some("#/components/schemas/Other")
        );
        let obj = Schema::object([
            ("b", Schema::string()),
            ("a", Schema::integer().format("int64")),
        ]);
        assert_eq!(obj.required, Some(vec!["a".to_string(), "b".to_string()]));
        assert_eq!(
            obj.properties.unwrap()["a"].format.as_deref(),
            Some("int64")
        );
        let p = Parameter::path("id", Schema::string());
        assert_eq!(p.required, Some(true));
        assert_eq!(p.location, ParameterLocation::Path);
    }
}
