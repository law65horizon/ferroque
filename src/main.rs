use ferroque::{api, worker::{HandlerRegistery, start_workers}, scheduler, AppState};

use sqlx::postgres::PgPoolOptions;
use dotenvy::dotenv;
use std::{env, time::Duration};

use crate::scheduler::run_scheduler;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!().run(&pool).await
        .expect("Failed to run migrations");

    let mut registry = HandlerRegistery::default();

    registry.register("send_email", |job| async move {
        let to = job.payload["to"].as_str().unwrap_or("unknown");
        tracing::info!("sending email to {to}");
        // api call
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    });

    registry.register("resize_image", |job| async move {
        let url = job.payload["url"].as_str().unwrap_or("unknown");
        tracing::info!("Resizing image at {url}");
        // api call

        if job.attempts == 1 {
            anyhow::bail!("Image service temporarily unavailable")
        }
        Ok(())
    });

    let scheduler_pool = pool.clone();
    tokio::spawn(async move {
        run_scheduler(scheduler_pool).await;
    });
    let worker_count = std::env::var("WORKER_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    start_workers(pool.clone(), registry, worker_count).await;

    let state = AppState { pool };
    let app = api::router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    tracing::info!("Ferroque listening on port 3000");
    axum::serve(listener, app).await.unwrap();
}