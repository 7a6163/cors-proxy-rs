use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn spawn_proxy(allowed_origins: Vec<String>) -> String {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .unwrap();

    let config = cors_proxy_rs::config::Config {
        port: 0,
        rate_limit_per_minute: 60,
        allowed_origins,
        max_body_size: 1024,
        block_private_ips: false, // disable for tests since mock server is on localhost
        timeout_secs: 5,
    };

    let state = cors_proxy_rs::proxy::AppState {
        client,
        config: config.clone(),
    };

    let app = Router::new()
        .fallback(cors_proxy_rs::proxy::proxy_handler)
        .layer(cors_proxy_rs::cors::CorsLayer::new(config.allowed_origins))
        .layer(cors_proxy_rs::rate_limit::RateLimitLayer::new(
            config.rate_limit_per_minute,
        ))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn test_proxy_get_request() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"message":"hello"}"#)
                .insert_header("content-type", "application/json"),
        )
        .mount(&mock_server)
        .await;

    let proxy_url = spawn_proxy(vec![]).await;
    let target = format!("{}/api/data", mock_server.uri());

    let resp = reqwest::Client::new()
        .get(format!("{proxy_url}/{target}"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("access-control-allow-origin")
            .unwrap()
            .to_str()
            .unwrap(),
        "*"
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["message"], "hello");
}

#[tokio::test]
async fn test_proxy_post_request() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/submit"))
        .respond_with(ResponseTemplate::new(201).set_body_string("created"))
        .mount(&mock_server)
        .await;

    let proxy_url = spawn_proxy(vec![]).await;
    let target = format!("{}/api/submit", mock_server.uri());

    let resp = reqwest::Client::new()
        .post(format!("{proxy_url}/{target}"))
        .body("test payload")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    assert!(resp.headers().contains_key("access-control-allow-origin"));
}

#[tokio::test]
async fn test_preflight_options() {
    let proxy_url = spawn_proxy(vec![]).await;

    let resp = reqwest::Client::new()
        .request(
            reqwest::Method::OPTIONS,
            format!("{proxy_url}/https://example.com/api"),
        )
        .header("origin", "https://myapp.com")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type, x-custom")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 204);
    assert_eq!(
        resp.headers()
            .get("access-control-allow-origin")
            .unwrap()
            .to_str()
            .unwrap(),
        "https://myapp.com"
    );
    assert!(resp.headers().get("access-control-allow-methods").is_some());
    assert_eq!(
        resp.headers()
            .get("access-control-allow-headers")
            .unwrap()
            .to_str()
            .unwrap(),
        "content-type, x-custom"
    );
}

#[tokio::test]
async fn test_cors_with_specific_origin() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&mock_server)
        .await;

    let proxy_url = spawn_proxy(vec![]).await;
    let target = format!("{}/test", mock_server.uri());

    let resp = reqwest::Client::new()
        .get(format!("{proxy_url}/{target}"))
        .header("origin", "https://myapp.com")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("access-control-allow-origin")
            .unwrap()
            .to_str()
            .unwrap(),
        "https://myapp.com"
    );
    assert_eq!(
        resp.headers()
            .get("access-control-allow-credentials")
            .unwrap()
            .to_str()
            .unwrap(),
        "true"
    );
}

#[tokio::test]
async fn test_origin_allowlist_blocks_unauthorized() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&mock_server)
        .await;

    let proxy_url = spawn_proxy(vec!["https://allowed.com".to_string()]).await;
    let target = format!("{}/test", mock_server.uri());

    let resp = reqwest::Client::new()
        .get(format!("{proxy_url}/{target}"))
        .header("origin", "https://evil.com")
        .send()
        .await
        .unwrap();

    // Request still proxied but no CORS origin header set (blocked by allowlist)
    assert!(resp.headers().get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn test_invalid_target_url() {
    let proxy_url = spawn_proxy(vec![]).await;

    let resp = reqwest::Client::new()
        .get(format!("{proxy_url}/not-a-valid-url"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_empty_target_url() {
    let proxy_url = spawn_proxy(vec![]).await;

    let resp = reqwest::Client::new()
        .get(format!("{proxy_url}/"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_body_too_large() {
    let proxy_url = spawn_proxy(vec![]).await;
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let target = format!("{}/upload", mock_server.uri());
    let large_body = "x".repeat(2048); // exceeds 1024 byte limit set in tests

    let resp = reqwest::Client::new()
        .post(format!("{proxy_url}/{target}"))
        .body(large_body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 413);
}
