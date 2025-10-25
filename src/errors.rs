use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Not Found")]
    NotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Bad request: {0}")]
    BadRequest(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Database(e) => {
                tracing::error!("Database error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Invalid or missing API key".to_string()),
            AppError::BadRequest(e) => (StatusCode::BAD_REQUEST, e.clone())
        };

        (status, Json(json!({ "error": message}))).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn not_found_returns_404() {
        let err = AppError::NotFound;
        let response = err.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[test]
    fn unauthorized_returns_401() {
        let err = AppError::Unauthorized;
        let response = err.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn bad_request_returns_400() {
        let err = AppError::BadRequest("job_type cannot be empty".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }
}