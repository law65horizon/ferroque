pub mod api;
pub mod db;
pub mod errors;
pub mod models;
pub mod worker;
pub mod scheduler;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::PgPool,
}