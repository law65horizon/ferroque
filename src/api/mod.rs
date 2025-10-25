pub mod jobs;
pub mod middleware;

use axum::{middleware as axum_middleware, routing::get, routing::post, Router};
use crate::{api::{middleware::require_api_key}, AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/jobs", post(jobs::enqueue_job))
        .route("/jobs", get(jobs::list_jobs))
        .route("/jobs/:id", get(jobs::get_job))
        .route("/jobs/:id", post(jobs::cancel_job))
        .layer(axum_middleware::from_fn(require_api_key))
        .with_state(state)
}