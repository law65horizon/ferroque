use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde_json::json;
use uuid::Uuid;
use crate::{
    db,
    errors::AppError,
    models::CreateJobRequest,
    AppState,
};

pub async fn enqueue_job(
    State(state): State<AppState>,
    Json(req): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    if req.job_type.trim().is_empty() {
        return Err(AppError::BadRequest("job_type cannot be empty".into()));
    }

    let job = db::insert_job(&state.pool, &req).await?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": job.id,
            "status": job.status,
            "job_type": job.job_type,
            "run_at": job.run_at,
        })),
    ))
}

pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let job = db::get_job(&state.pool, id).await?;

    Ok(Json(serde_json::json!({
        "id": job.id,
        "job_type": job.job_type,
        "payload": job.payload,
        "status": job.status,
        "attempts": job.attempts,
        "max_attempts": job.max_attempts,
        "error": job.error,
        "run_at": job.run_at,
        "created_at": job.created_at,
    })))
}

pub async fn list_jobs(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let jobs = db::list_jobs(&state.pool).await?;

    Ok(Json(serde_json::json!({ "jobs": jobs, "count": jobs.len() })))
}

pub async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>
) -> Result<Json<serde_json::Value>, AppError> {
    let job = db::cancel_job(&state.pool, id).await?;

    Ok(Json(json!({"id": job.id, "status": job.status})))
}