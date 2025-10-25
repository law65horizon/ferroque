use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response
};

use std::env;

pub async fn require_api_key(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let valid_key = env::var("API_KEY").expect("API_KEY must be set");

    let provided_key = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    match provided_key {
        Some(key) if key == valid_key => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED)
    }
}