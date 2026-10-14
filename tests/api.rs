//! End-to-end tests: each test spins up the full router on an in-memory
//! SQLite database and sends real HTTP requests through it.

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use snip_api::{app, config::Config, db, state::AppState};
use tower::ServiceExt;

const BASE_URL: &str = "http://sn.ip";

async fn test_app() -> Router {
    test_app_with_static(None).await
}

async fn test_app_with_static(static_dir: Option<String>) -> Router {
    let config = Config {
        database_url: "sqlite::memory:".into(),
        jwt_secret: "test-secret-that-is-at-least-32-characters".into(),
        jwt_ttl_hours: 1,
        base_url: BASE_URL.into(),
        port: 0,
        static_dir,
    };
    let pool = db::connect(&config.database_url).await.unwrap();
    app(AppState {
        db: pool,
        config: Arc::new(config),
    })
}

/// Sends a request and returns (status, headers, JSON body or Null).
async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let req = match body {
        Some(json) => req
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, headers, json)
}

async fn register(app: &Router, email: &str) -> String {
    let (status, _, body) = send(
        app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": email, "password": "super-secret-pw" })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    body["token"].as_str().unwrap().to_string()
}

async fn create_link(app: &Router, token: &str, body: Value) -> Value {
    let (status, _, link) = send(app, "POST", "/api/links", Some(token), Some(body)).await;
    assert_eq!(status, StatusCode::CREATED, "{link}");
    link
}

#[tokio::test]
async fn health_check() {
    let app = test_app().await;
    let (status, _, body) = send(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn register_login_and_me() {
    let app = test_app().await;
    register(&app, "Ada@Example.com").await;

    // Duplicate email (case-insensitive) is rejected.
    let (status, _, body) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": "ada@example.com", "password": "another-password" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"]["code"], "conflict");

    // Wrong password.
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": "ada@example.com", "password": "wrong-password" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Correct password.
    let (status, _, body) = send(
        &app,
        "POST",
        "/api/auth/login",
        None,
        Some(json!({ "email": "ada@example.com", "password": "super-secret-pw" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let token = body["token"].as_str().unwrap();

    let (status, _, me) = send(&app, "GET", "/api/auth/me", Some(token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["email"], "ada@example.com");
}

#[tokio::test]
async fn rejects_invalid_input() {
    let app = test_app().await;
    let (status, _, body) = send(
        &app,
        "POST",
        "/api/auth/register",
        None,
        Some(json!({ "email": "not-an-email", "password": "short" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body["error"]["code"], "validation_error");

    let token = register(&app, "bob@example.com").await;
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/links",
        Some(&token),
        Some(json!({ "target_url": "javascript:alert(1)" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    let (status, _, _) = send(
        &app,
        "POST",
        "/api/links",
        Some(&token),
        Some(json!({ "target_url": "https://example.com", "code": "docs" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn protected_routes_require_a_valid_token() {
    let app = test_app().await;
    let (status, _, _) = send(&app, "GET", "/api/links", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let (status, _, _) = send(&app, "GET", "/api/links", Some("garbage"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn link_lifecycle() {
    let app = test_app().await;
    let token = register(&app, "carol@example.com").await;

    // Generated code.
    let link = create_link(
        &app,
        &token,
        json!({ "target_url": "https://www.rust-lang.org" }),
    )
    .await;
    let code = link["code"].as_str().unwrap();
    assert_eq!(code.len(), 7);
    assert_eq!(link["short_url"], format!("{BASE_URL}/{code}"));

    // Custom code, and the same code can't be taken twice.
    let custom = create_link(
        &app,
        &token,
        json!({ "target_url": "https://doc.rust-lang.org/book/", "code": "rust-book" }),
    )
    .await;
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/links",
        Some(&token),
        Some(json!({ "target_url": "https://example.com", "code": "rust-book" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // List with pagination, newest first.
    let (status, _, page) = send(&app, "GET", "/api/links?per_page=1", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page["total"], 2);
    assert_eq!(page["items"].as_array().unwrap().len(), 1);
    assert_eq!(page["items"][0]["code"], "rust-book");

    // Update.
    let id = custom["id"].as_i64().unwrap();
    let (status, _, updated) = send(
        &app,
        "PATCH",
        &format!("/api/links/{id}"),
        Some(&token),
        Some(json!({ "target_url": "https://doc.rust-lang.org/rust-by-example/" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        updated["target_url"],
        "https://doc.rust-lang.org/rust-by-example/"
    );
    assert_eq!(updated["is_active"], true);

    // Delete.
    let (status, _, _) = send(
        &app,
        "DELETE",
        &format!("/api/links/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _, _) = send(&app, "GET", &format!("/api/links/{id}"), Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn users_cannot_see_each_others_links() {
    let app = test_app().await;
    let alice = register(&app, "alice@example.com").await;
    let mallory = register(&app, "mallory@example.com").await;

    let link = create_link(&app, &alice, json!({ "target_url": "https://example.com" })).await;
    let id = link["id"].as_i64().unwrap();

    for (method, uri) in [
        ("GET", format!("/api/links/{id}")),
        ("GET", format!("/api/links/{id}/stats")),
        ("DELETE", format!("/api/links/{id}")),
    ] {
        let (status, _, _) = send(&app, method, &uri, Some(&mallory), None).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{method} {uri}");
    }

    let (_, _, page) = send(&app, "GET", "/api/links", Some(&mallory), None).await;
    assert_eq!(page["total"], 0);
}

#[tokio::test]
async fn redirect_records_clicks_and_stats() {
    let app = test_app().await;
    let token = register(&app, "dave@example.com").await;
    let link = create_link(
        &app,
        &token,
        json!({ "target_url": "https://example.com/landing", "code": "promo" }),
    )
    .await;

    for referrer in [
        Some("https://twitter.com"),
        Some("https://twitter.com"),
        None,
    ] {
        let mut req = Request::builder().uri("/promo");
        if let Some(r) = referrer {
            req = req.header(header::REFERER, r);
        }
        let res = app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::TEMPORARY_REDIRECT);
        assert_eq!(
            res.headers()[header::LOCATION],
            "https://example.com/landing"
        );
    }

    let id = link["id"].as_i64().unwrap();
    let (status, _, stats) = send(
        &app,
        "GET",
        &format!("/api/links/{id}/stats"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(stats["total_clicks"], 3);
    assert!(stats["last_clicked_at"].is_string());
    assert_eq!(stats["daily"][0]["clicks"], 3);
    assert_eq!(stats["top_referrers"][0]["referrer"], "https://twitter.com");
    assert_eq!(stats["top_referrers"][0]["clicks"], 2);

    // Unknown code.
    let (status, _, _) = send(&app, "GET", "/nope-nope", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn disabled_and_expired_links_return_gone() {
    let app = test_app().await;
    let token = register(&app, "erin@example.com").await;

    let link = create_link(
        &app,
        &token,
        json!({ "target_url": "https://example.com", "code": "paused" }),
    )
    .await;
    let id = link["id"].as_i64().unwrap();
    send(
        &app,
        "PATCH",
        &format!("/api/links/{id}"),
        Some(&token),
        Some(json!({ "is_active": false })),
    )
    .await;
    let (status, _, body) = send(&app, "GET", "/paused", None, None).await;
    assert_eq!(status, StatusCode::GONE);
    assert_eq!(body["error"]["code"], "gone");

    // Expiry dates must be in the future when creating.
    let (status, _, _) = send(
        &app,
        "POST",
        "/api/links",
        Some(&token),
        Some(json!({ "target_url": "https://example.com", "expires_at": "2000-01-01T00:00:00Z" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn serves_openapi_docs() {
    let app = test_app().await;
    let (status, _, spec) = send(&app, "GET", "/openapi.json", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(spec["paths"]["/api/links"].is_object());
    assert!(spec["components"]["securitySchemes"]["bearer"].is_object());

    let (status, _, _) = send(&app, "GET", "/docs", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn serves_dashboard_alongside_short_links() {
    let dir = std::env::temp_dir().join(format!("snip-static-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("assets")).unwrap();
    std::fs::write(dir.join("index.html"), "<!doctype html><title>snip</title>").unwrap();
    std::fs::write(dir.join("assets/app.js"), "console.log(1)").unwrap();

    let app = test_app_with_static(Some(dir.to_string_lossy().into_owned())).await;

    for uri in ["/", "/links/42", "/assets/app.js"] {
        let res = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK, "{uri}");
    }

    // Short links still redirect, and the API still answers.
    let token = register(&app, "frank@example.com").await;
    create_link(
        &app,
        &token,
        json!({ "target_url": "https://example.com", "code": "promo" }),
    )
    .await;
    let (status, headers, _) = send(&app, "GET", "/promo", None, None).await;
    assert_eq!(status, StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(headers[header::LOCATION], "https://example.com");

    let (status, _, _) = send(&app, "GET", "/health", None, None).await;
    assert_eq!(status, StatusCode::OK);

    std::fs::remove_dir_all(dir).ok();
}
