use crate::{db, models::Job};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use sqlx::PgPool;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};
// use tracing_subscriber::registry;

pub type HandlerFn = Arc<
  dyn Fn(Job) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>> 
  + Send
  + Sync
>;

#[derive(Clone, Default)]
pub struct HandlerRegistery {
    handlers: HashMap<String, HandlerFn>
}

impl HandlerRegistery {
    pub fn register<F, Fut>(&mut self, job_type: &str, f: F)
    where 
      F: Fn(Job) -> Fut + Send + Sync + 'static,
      Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    {
        self.handlers.insert(
            job_type.to_string(), 
            Arc::new(move |job| Box::pin(f(job)))
        );
    }

    pub fn get(&self, job_type: &str) -> Option<HandlerFn> {
        self.handlers.get(job_type).cloned()
    }
}

pub async fn run_worker(
    pool: PgPool,
    registry: Arc<HandlerRegistery>,
    worker_id: usize
) {
    info!("Worker {worker_id} started");

    loop {
        match db::claim_job(&pool).await {
            Ok(Some(job)) => {
                let job_id = job.id;
                let job_type = job.job_type.clone();
                info!("Worker {worker_id} claimed job {job_id} (type: {job_type})");

                match registry.get(&job_type) {
                    Some(handler) => {
                        match handler(job).await {
                            Ok(()) => {
                                info!("Job {job_id} succeeded");
                                if let Err(e) = db::resolve_job(&pool, job_id, true, true, None).await {
                                    error!("Failed to mark job {job_id} succeeded: {e}")
                                }
                            }
                            Err(e) => {
                                warn!("Job {job_id} failed: {e}");
                                if let Err(db_err) = db::resolve_job(&pool, job_id, false, false, Some(e.to_string())).await {
                                    error!("Failed to mark job {job_id} failed: {db_err}")
                                }
                            }
                        }
                    }
                    None => {
                        warn!("No handler registered for job type '{job_type}', marking dead");
                        db::resolve_job(
                            &pool, 
                            job_id, 
                            false, 
                            true,
                            Some(format!("No handler for job type '{job_type}'"))
                        ).await.ok();
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                error!("Worker {worker_id} error claiming job: {e}");
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
}

pub async fn start_workers(
    pool: PgPool,
    registry: HandlerRegistery,
    count: usize,
) {
    let registry = Arc::new(registry);

    for id in 0..count {
        let pool = pool.clone();
        let registry = Arc::clone(&registry);

        tokio::spawn(async move {
            run_worker(pool, registry, id).await;
        });
    }

    info!("started {count} workers")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    fn make_test_job(job_type: &str) -> Job {
        Job { 
            id: Uuid::new_v4(), 
            job_type: job_type.to_string(), 
            payload: json!({}), 
            status: crate::models::JobStatus::Pending, 
            priority: 0, 
            attempts: 0, 
            max_attempts: 3, 
            error: None, 
            run_at: time::OffsetDateTime::now_utc(), 
            created_at: time::OffsetDateTime::now_utc(), 
            updated_at: time::OffsetDateTime::now_utc() 
        }
    }

    #[test]
    fn registry_returns_none_for_unregistered_type() {
        let registry = HandlerRegistery::default();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn registry_returns_handler_after_registration() {
        let mut registry = HandlerRegistery::default();

        registry.register("send_email", |job| async move {
            Ok(())
        });
        assert!(registry.get("send_email").is_some())

    }

    #[test]
    fn registry_handles_multiple_job_types() {
        let mut registry = HandlerRegistery::default();
        registry.register("send_email", |job| async move {
            Ok(())
        });
        registry.register("resize_image", |job| async move {
            Ok(())
        });
        registry.register("sync_crm", |job| async move {
            Ok(())
        });

        assert!(registry.get("send_email").is_some());
        assert!(registry.get("resize_image").is_some());
        assert!(registry.get("sync_crm").is_some());
        assert!(registry.get("unknown").is_none());
    }

    #[tokio::test]
    async fn registered_handler_executes_successfully() {
        let mut registry = HandlerRegistery::default();
        registry.register("test_job", |_job| async move { Ok(()) });

        let handler = registry.get("test_job").unwrap();
        let job = make_test_job("test_job");
        let result = handler(job).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn handler_can_return_error() {
        let mut registry = HandlerRegistery::default();
        registry.register("failing_job", |_job| async move {
            anyhow::bail!("something went wrong")
        });

        let handler = registry.get("failing_job").unwrap();
        let job = make_test_job("failing_job");
        let result = handler(job).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "something went wrong");
    }
}