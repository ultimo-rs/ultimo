#![cfg(feature = "testing")]

use std::net::IpAddr;
use ultimo::guard::{Decision, GeoFilter, Guard};
use ultimo::testing::TestClient;
use ultimo::{Context, Ultimo};

fn app_with(mw: ultimo::middleware::BoxedMiddleware, trust_proxy: bool) -> Ultimo {
    let mut app = Ultimo::new_without_defaults();
    app.trust_proxy(trust_proxy);
    app.use_middleware(mw);
    app.get("/", |ctx: Context| async move {
        let country = ctx.country().await.unwrap_or_else(|| "none".into());
        ctx.text(format!("ok country={country}")).await
    });
    app.get(
        "/admin",
        |ctx: Context| async move { ctx.text("admin").await },
    );
    app
}

// ---------------------------------------------------------------- Guard

fn block_scanners() -> Guard {
    Guard::new(|ctx: &Context| {
        let ua = ctx.req.header("user-agent").unwrap_or_default();
        Decision::deny_if(ua.contains("sqlmap") || ua.contains("nikto"))
    })
}

#[tokio::test]
async fn guard_allows_and_denies_by_predicate() {
    let client = TestClient::new(app_with(block_scanners().build(), false));

    let ok = client
        .get("/")
        .header("user-agent", "Mozilla/5.0")
        .send()
        .await;
    assert_eq!(ok.status(), 200);

    let blocked = client
        .get("/")
        .header("user-agent", "sqlmap/1.8")
        .send()
        .await;
    assert_eq!(blocked.status(), 403);
    assert_eq!(blocked.text(), "Forbidden");
}

#[tokio::test]
async fn guard_can_send_a_custom_response() {
    let guard = Guard::new(|ctx: &Context| {
        if ctx.req.path().starts_with("/admin") && ctx.req.header("x-admin").is_none() {
            Decision::respond(404, "Not Found")
        } else {
            Decision::Allow
        }
    });
    let client = TestClient::new(app_with(guard.build(), false));

    let hidden = client.get("/admin").send().await;
    assert_eq!(hidden.status(), 404);
    assert_eq!(hidden.text(), "Not Found");
    assert_eq!(
        client
            .get("/admin")
            .header("x-admin", "1")
            .send()
            .await
            .text(),
        "admin"
    );
    assert_eq!(client.get("/").send().await.status(), 200);
}

#[tokio::test]
async fn guard_deny_status_is_configurable() {
    let guard = Guard::new(|_: &Context| Decision::Deny).deny_status(451, "Unavailable");
    let res = TestClient::new(app_with(guard.build(), false))
        .get("/")
        .send()
        .await;
    assert_eq!(res.status(), 451);
    assert_eq!(res.text(), "Unavailable");
}

#[tokio::test]
async fn guard_types_are_also_in_middleware_builtin() {
    use ultimo::middleware::builtin;
    let guard = builtin::Guard::new(|_: &Context| builtin::Decision::Deny);
    let res = TestClient::new(app_with(guard.build(), false))
        .get("/")
        .send()
        .await;
    assert_eq!(res.status(), 403);
    let _ = builtin::GeoFilter::deny(&["KP"]);
}

#[test]
fn decision_helpers() {
    assert_eq!(Decision::deny_if(true), Decision::Deny);
    assert_eq!(Decision::deny_if(false), Decision::Allow);
    assert_eq!(Decision::allow_if(true), Decision::Allow);
    assert_eq!(Decision::allow_if(false), Decision::Deny);
}

// ---------------------------------------------------------------- GeoFilter

fn lookup(ip: IpAddr) -> Option<String> {
    match ip.to_string().as_str() {
        "203.0.113.1" => Some("us".into()),
        "203.0.113.2" => Some("KP".into()),
        _ => None,
    }
}

async fn get_from(client: &TestClient, ip: &str) -> ultimo::testing::TestResponse {
    client.get("/").header("x-forwarded-for", ip).send().await
}

#[tokio::test]
async fn geo_allow_list_with_lookup() {
    let geo = GeoFilter::allow(&["US", "CA"]).lookup(lookup);
    let client = TestClient::new(app_with(geo.build(), true));

    let us = get_from(&client, "203.0.113.1").await;
    assert_eq!(us.status(), 200);
    assert_eq!(
        us.text(),
        "ok country=US",
        "country is normalized and exposed"
    );
    assert_eq!(get_from(&client, "203.0.113.2").await.status(), 403);
    // Unknown country fails closed on an allow-list.
    assert_eq!(get_from(&client, "198.51.100.9").await.status(), 403);
}

#[tokio::test]
async fn geo_deny_list_lets_unknown_through_by_default() {
    let geo = GeoFilter::deny(&["kp"]).lookup(lookup);
    let client = TestClient::new(app_with(geo.build(), true));

    assert_eq!(get_from(&client, "203.0.113.1").await.status(), 200);
    assert_eq!(get_from(&client, "203.0.113.2").await.status(), 403);
    let unknown = get_from(&client, "198.51.100.9").await;
    assert_eq!(unknown.status(), 200);
    assert_eq!(unknown.text(), "ok country=none");

    let strict = GeoFilter::deny(&["KP"]).lookup(lookup).deny_unknown();
    let client = TestClient::new(app_with(strict.build(), true));
    assert_eq!(get_from(&client, "198.51.100.9").await.status(), 403);

    let lenient = GeoFilter::allow(&["US"]).lookup(lookup).allow_unknown();
    let client = TestClient::new(app_with(lenient.build(), true));
    assert_eq!(get_from(&client, "198.51.100.9").await.status(), 200);
}

#[tokio::test]
async fn geo_cdn_header_is_only_trusted_behind_a_trusted_proxy() {
    let geo = || GeoFilter::allow(&["US"]).header("CF-IPCountry");

    let trusted = TestClient::new(app_with(geo().build(), true));
    let res = trusted.get("/").header("cf-ipcountry", "US").send().await;
    assert_eq!(res.status(), 200);
    assert_eq!(res.text(), "ok country=US");
    let res = trusted.get("/").header("cf-ipcountry", "DE").send().await;
    assert_eq!(res.status(), 403);
    // Cloudflare's "XX" means unknown.
    let res = trusted.get("/").header("cf-ipcountry", "XX").send().await;
    assert_eq!(res.status(), 403);

    // Without trust_proxy a client could set the header itself, so it's ignored.
    let untrusted = TestClient::new(app_with(geo().build(), false));
    let res = untrusted.get("/").header("cf-ipcountry", "US").send().await;
    assert_eq!(res.status(), 403);
}

#[tokio::test]
async fn geo_header_falls_back_to_lookup() {
    let geo = GeoFilter::deny(&["KP"])
        .header("CF-IPCountry")
        .lookup(lookup);
    let client = TestClient::new(app_with(geo.build(), true));

    // No header → lookup by client IP.
    assert_eq!(get_from(&client, "203.0.113.2").await.status(), 403);
    // Header wins when present.
    let res = client
        .get("/")
        .header("x-forwarded-for", "203.0.113.2")
        .header("cf-ipcountry", "FR")
        .send()
        .await;
    assert_eq!(res.text(), "ok country=FR");
}

#[test]
#[should_panic(expected = "invalid country code")]
fn geo_rejects_bad_country_codes() {
    let _ = GeoFilter::allow(&["USA"]);
}

#[test]
#[should_panic(expected = "needs a country source")]
fn geo_requires_a_source() {
    let _ = GeoFilter::allow(&["US"]).build();
}
