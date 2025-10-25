use sqlx::PgPool;
use tokio::time::{interval, Duration};
use tracing::{error, info};

pub async fn run_scheduler(pool: PgPool) {
    info!("Scheduler started");
    let mut tick = interval(Duration::from_secs(30));

    loop {
        tick.tick().await;

        match reap_stuck_jobs(&pool).await {
            Ok(0) => {}
            Ok(n) => info!("Schedulered requeued {n} stuck jobs"),
            Err(e) => error!("Schedulered error during reap: {e}"),
        }
    }
}

async fn reap_stuck_jobs(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET 
            status = CASE
               WHEN attempts >= max_attempts THEN 'dead'::job_status
               ELSE 'pending'::job_status
            END,
            error = 'Worker timeout - requeued by scheduler',
            run_at = NOW(),
            updated_at = NOW()
        WHERE status = 'running'
        AND updated_at < NOW() - INTERVAL '5 minutes'
        "#
    ).execute(pool).await?;

    Ok(result.rows_affected())
}