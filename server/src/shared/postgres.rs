//! PostgreSQL connection pool setup.

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Creates a PostgreSQL connection pool using `DATABASE_URL` from the environment, then runs any
/// pending migrations from `migrations/` so the schema is always up to date on startup.
pub async fn create_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set (see .env.example)");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run database migrations");

    pool
}
