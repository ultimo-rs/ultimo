#![cfg(feature = "testing")]

use ultimo::openapi::OpenApiBuilder;
use ultimo::testing::TestClient;
use ultimo::Ultimo;

#[tokio::test]
async fn serve_docs_returns_swagger_ui_html() {
    let mut app = Ultimo::new_without_defaults();
    let spec = OpenApiBuilder::new()
        .title("Test API")
        .version("1.0.0")
        .build();
    app.serve_docs("/docs", spec);

    let client = TestClient::new(app);
    let res = client.get("/docs").send().await;

    assert_eq!(res.status(), 200);
    let body = res.text();
    assert!(body.contains("swagger"), "Should contain Swagger UI");
    assert!(
        body.contains("/docs/openapi.json"),
        "Should reference spec URL"
    );
}

#[tokio::test]
async fn serve_docs_returns_openapi_json() {
    let mut app = Ultimo::new_without_defaults();
    let spec = OpenApiBuilder::new()
        .title("My Test API")
        .version("2.0.0")
        .description("A test API")
        .build();
    app.serve_docs("/docs", spec);

    let client = TestClient::new(app);
    let res = client.get("/docs/openapi.json").send().await;

    assert_eq!(res.status(), 200);
    let body = res.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Should be valid JSON");
    assert_eq!(json["info"]["title"], "My Test API");
    assert_eq!(json["info"]["version"], "2.0.0");
    assert_eq!(json["openapi"], "3.0.0");
}

#[tokio::test]
async fn serve_docs_custom_path() {
    let mut app = Ultimo::new_without_defaults();
    let spec = OpenApiBuilder::new()
        .title("Custom")
        .version("1.0.0")
        .build();
    app.serve_docs("/api-docs", spec);

    let client = TestClient::new(app);

    let res = client.get("/api-docs").send().await;
    assert_eq!(res.status(), 200);

    let res = client.get("/api-docs/openapi.json").send().await;
    assert_eq!(res.status(), 200);
}
