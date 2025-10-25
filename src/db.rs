use sqlx::PgPool;
use uuid::Uuid;
use crate::{errors::AppError, models::{CreateJobRequest, Job}};

pub async fn insert_job(
    pool: &PgPool,
    req: &CreateJobRequest,
) -> Result<Job, AppError> {
    let job = sqlx::query_as!(
        Job,
        r#"
        INSERT INTO jobs (job_type, payload, priority, max_attempts, run_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING 
           id, job_type, payload,
           status AS "status: _",
           priority, attempts, max_attempts,
           error, run_at, created_at, updated_at
        "#,
        req.job_type,
        req.payload.clone().unwrap_or(serde_json::json!({})),
        req.priority.unwrap_or(0),
        req.max_attempts.unwrap_or(3),
        req.run_at.unwrap_or_else(time::OffsetDateTime::now_utc),
    ).fetch_one(pool).await?;

    Ok(job)

}

pub async fn get_job(pool: &PgPool, id: Uuid) -> Result<Job, AppError> {
    let job = sqlx::query_as!(
        Job,
        r#"
        SELECT 
           id, job_type, payload,
           status As "status: _",
           priority, attempts, max_attempts,
           error, run_at, created_at, updated_at
        FROM jobs
        WHERE id = $1
        "#,
        id
    ).fetch_optional(pool).await?.ok_or(AppError::NotFound)?;

    Ok(job)
}

pub async fn list_jobs(pool: &PgPool) -> Result<Vec<Job>, AppError> {
    let jobs = sqlx::query_as!(
        Job,
        r#"
        SELECT 
           id, job_type, payload,
           status As "status: _",
           priority, attempts, max_attempts,
           error, run_at, created_at, updated_at
        FROM jobs
        ORDER BY created_at DESC
        LIMIT 100
        "#
    ).fetch_all(pool).await?;

    Ok(jobs)
}

pub async fn claim_job(pool: &PgPool) -> Result<Option<Job>, AppError> {
    let job = sqlx::query_as!(
        Job,
        r#"
        UPDATE jobs
        SET 
           status = 'running',
           attempts = attempts + 1,
           updated_at = NOW()
        WHERE id = (
        SELECT id FROM jobs
        WHERE status = 'pending'
          AND run_at <= NOW()
        ORDER BY priority DESC, run_at ASC
        LIMIT 1
        FOR UPDATE SKIP LOCKED
        )
        RETURNING 
           id, job_type, payload,
           status As "status: _",
           priority, attempts, max_attempts,
           error, run_at, created_at, updated_at
        "#
    ).fetch_optional(pool).await?;

    Ok(job)
}

pub async fn resolve_job(
    pool: &PgPool,
    id: Uuid,
    success: bool,
    dead: bool,
    error_msg: Option<String>
) -> Result<(), AppError> {
    let job = sqlx::query!(
        r#"
        UPDATE jobs
        SET 
          status = CASE
            WHEN $1 THEN 'succeeded'::job_status
            WHEN attempts >= max_attempts OR $2 THEN 'dead'::job_status
            ELSE 'pending'::job_status
          END,
          error = $3,
          run_at = CASE
            WHEN $1 THEN run_at
            ELSE NOW() + (INTERVAL '1 second' * POWER(2, attempts))
          END,
          updated_at = NOW()
        WHERE id = $4
        "#,
        success,
        dead,
        error_msg,
        id
    ).execute(pool).await?;

    Ok(())
}

pub async fn cancel_job(pool: &PgPool, id: Uuid) -> Result<Job, AppError> {
    let job = sqlx::query_as!(
        Job,
        r#"
        UPDATE jobs
        SET status = 'dead'::job_status, updated_at = NOW()
        WHERE id = $1 AND status = 'pending'
        RETURNING
           id, job_type, payload,
           status As "status: _",
           priority, attempts, max_attempts,
           error, run_at, created_at, updated_at
        "#,
        id
    ).fetch_optional(pool).await?.ok_or(AppError::NotFound)?;

    Ok(job)
}