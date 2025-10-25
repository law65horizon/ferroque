use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use ferroque::{api, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;
use http_body_util::BodyExt;

// Helper: build the router with a test state
async fn test_app() -> axum::Router {
    // Use a real DB for integration tests — read from env
    dotenvy::from_filename(".env.test").ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/ferroque".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    sqlx::migrate!().run(&pool).await.expect("Migrations failed");

    let state = AppState { pool };
    api::router(state)
}

// Helper: parse response body as JSON
async fn body_json(body: axum::body::Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

// ── Auth tests ────────────────────────────────────────────

#[tokio::test]
async fn missing_api_key_returns_401() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_api_key_returns_401() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/jobs")
                .header("x-api-key", "wrong-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

// ── Job enqueue tests ─────────────────────────────────────

#[tokio::test]
async fn enqueue_valid_job_returns_201() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("x-api-key", "test-secret")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"job_type": "send_email", "payload": {"to": "test@example.com"}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = body_json(response.into_body()).await;
    assert_eq!(body["status"], "Pending");
    assert_eq!(body["job_type"], "send_email");
    assert!(body["id"].is_string());
}

#[tokio::test]
async fn enqueue_empty_job_type_returns_400() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("x-api-key", "test-secret")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"job_type": "   "}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn enqueue_missing_job_type_returns_422() {
    let app = test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("x-api-key", "test-secret")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"payload": {}}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 when JSON doesn't match the expected shape
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Job retrieval tests ───────────────────────────────────

#[tokio::test]
async fn get_nonexistent_job_returns_404() {
    let app = test_app().await;

    let fake_id = uuid::Uuid::new_v4();
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/jobs/{fake_id}"))
                .header("x-api-key", "test-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn enqueue_then_retrieve_job() {
    let app = test_app().await;

    // Enqueue
    let enqueue_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jobs")
                .header("x-api-key", "test-secret")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"job_type": "test_retrieve", "payload": {"x": 42}})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let created = body_json(enqueue_response.into_body()).await;
    let id = created["id"].as_str().unwrap();

    // Retrieve
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/jobs/{id}"))
                .header("x-api-key", "test-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);
    let body = body_json(get_response.into_body()).await;
    assert_eq!(body["id"], id);
    assert_eq!(body["job_type"], "test_retrieve");
    assert_eq!(body["payload"]["x"], 42);
    assert_eq!(body["status"], "Pending");
}