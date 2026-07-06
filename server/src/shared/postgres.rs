//! PostgreSQL connection pool setup.

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Creates a PostgreSQL connection pool using `DATABASE_URL` from the environment.
pub async fn create_pool() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set (see .env.example)");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to Postgres")
}
