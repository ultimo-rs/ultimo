#![cfg(all(feature = "metrics", feature = "testing"))]

use std::time::Duration;
use ultimo::metrics::Metrics;
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo, UltimoError};

fn app_with(metrics: &Metrics) -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.use_middleware(metrics.middleware());
    app.get("/users/:id", |ctx: Context| async move {
        ctx.text(ctx.req.param("id")?.to_string()).await
    });
    app.post("/users", |ctx: Context| async move {
        ctx.status(201).await;
        ctx.text("created").await
    });
    app.get("/boom", |_ctx: Context| async move {
        Err::<ultimo::response::Response, _>(UltimoError::Internal("boom".into()))
    });
    app.get("/route", |ctx: Context| async move {
        ctx.text(ctx.req.route_pattern().unwrap_or("none").to_string())
            .await
    });
    app.get("/metrics", metrics.handler());
    app
}

fn line_value(body: &str, prefix: &str) -> Option<f64> {
    body.lines()
        .find(|l| l.starts_with(prefix))
        .and_then(|l| l.rsplit(' ').next())
        .and_then(|v| v.parse().ok())
}

#[tokio::test]
async fn counts_requests_by_route_pattern_not_raw_path() {
    let metrics = Metrics::new();
    let client = TestClient::new(app_with(&metrics));

    client.get("/users/1").send().await;
    client.get("/users/2").send().await;
    client.post("/users").send().await;

    let body = metrics.render();
    assert_eq!(
        line_value(
            &body,
            r#"http_requests_total{method="GET",route="/users/:id",status="200"}"#
        ),
        Some(2.0),
        "{body}"
    );
    assert_eq!(
        line_value(
            &body,
            r#"http_requests_total{method="POST",route="/users",status="201"}"#
        ),
        Some(1.0),
        "{body}"
    );
    assert!(!body.contains("/users/1"), "raw paths must not be labels");
}

#[tokio::test]
async fn handler_errors_are_recorded_with_their_status() {
    let metrics = Metrics::new();
    let client = TestClient::new(app_with(&metrics));

    let res = client.get("/boom").send().await;
    assert_eq!(res.status(), 500);

    let body = metrics.render();
    assert_eq!(
        line_value(
            &body,
            r#"http_requests_total{method="GET",route="/boom",status="500"}"#
        ),
        Some(1.0),
        "{body}"
    );
}

#[tokio::test]
async fn histogram_has_cumulative_buckets_sum_and_count() {
    let metrics = Metrics::new();
    let client = TestClient::new(app_with(&metrics));

    client.get("/users/1").send().await;
    client.get("/users/1").send().await;

    let body = metrics.render();
    assert!(body.contains("# TYPE http_request_duration_seconds histogram"));
    let prefix =
        r#"http_request_duration_seconds_bucket{method="GET",route="/users/:id",le="+Inf"}"#;
    assert_eq!(line_value(&body, prefix), Some(2.0), "{body}");
    let count = r#"http_request_duration_seconds_count{method="GET",route="/users/:id"}"#;
    assert_eq!(line_value(&body, count), Some(2.0), "{body}");
    let sum = r#"http_request_duration_seconds_sum{method="GET",route="/users/:id"}"#;
    assert!(line_value(&body, sum).unwrap() >= 0.0);

    // Buckets are cumulative: each `le` is >= the previous one.
    let mut last = 0.0;
    for line in body
        .lines()
        .filter(|l| l.starts_with("http_request_duration_seconds_bucket{method=\"GET\""))
    {
        let v: f64 = line.rsplit(' ').next().unwrap().parse().unwrap();
        assert!(v >= last, "{body}");
        last = v;
    }
}

#[tokio::test]
async fn custom_buckets_are_used() {
    let metrics = Metrics::with_buckets(&[0.5, 1.0]);
    let client = TestClient::new(app_with(&metrics));
    client.get("/users/1").send().await;

    let body = metrics.render();
    assert!(body.contains(r#"le="0.5""#), "{body}");
    assert!(body.contains(r#"le="1""#), "{body}");
    assert!(!body.contains(r#"le="0.005""#), "{body}");
}

#[tokio::test]
async fn metrics_endpoint_serves_prometheus_text_format() {
    let metrics = Metrics::new();
    let client = TestClient::new(app_with(&metrics));
    client.get("/users/7").send().await;

    let res = client.get("/metrics").send().await;
    assert_eq!(res.status(), 200);
    assert_eq!(
        res.header("content-type"),
        Some("text/plain; version=0.0.4; charset=utf-8")
    );
    let body = res.text();
    assert!(
        body.contains("# TYPE http_requests_total counter"),
        "{body}"
    );
    assert!(
        body.contains("# TYPE http_requests_in_flight gauge"),
        "{body}"
    );
    assert!(body.contains(r#"route="/users/:id""#), "{body}");
}

#[tokio::test]
async fn in_flight_returns_to_zero_even_when_request_times_out() {
    let metrics = Metrics::new();
    let mut app = Ultimo::new_without_defaults();
    app.request_timeout(Duration::from_millis(20));
    app.use_middleware(metrics.middleware());
    app.get("/slow", |ctx: Context| async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        ctx.text("late").await
    });
    let client = TestClient::new(app);

    let res = client.get("/slow").send().await;
    assert_eq!(res.status(), 408);
    let body = metrics.render();
    assert_eq!(
        line_value(&body, "http_requests_in_flight "),
        Some(0.0),
        "{body}"
    );
}

#[tokio::test]
async fn route_pattern_is_exposed_on_request() {
    let metrics = Metrics::new();
    let client = TestClient::new(app_with(&metrics));
    assert_eq!(client.get("/route").send().await.text(), "/route");
}
