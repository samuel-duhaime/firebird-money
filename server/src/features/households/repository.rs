use sqlx::PgPool;

use super::model::Household;

const SELECT_COLUMNS: &str = "id, created_at";

/// Creates a new, empty household and returns it.
pub async fn create(pool: &PgPool) -> Result<Household, sqlx::Error> {
    sqlx::query_as::<_, Household>(&format!(
        "INSERT INTO households DEFAULT VALUES RETURNING {SELECT_COLUMNS}"
    ))
    .fetch_one(pool)
    .await
}

/// Fetches a single household by id, or `None` if it doesn't exist.
pub async fn get(pool: &PgPool, id: i32) -> Result<Option<Household>, sqlx::Error> {
    sqlx::query_as::<_, Household>(&format!(
        "SELECT {SELECT_COLUMNS} FROM households WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Deletes a household by id. Returns `true` if a row was deleted, `false` if the id didn't exist.
pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM households WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
