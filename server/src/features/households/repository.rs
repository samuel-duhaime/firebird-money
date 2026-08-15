use sqlx::PgPool;
use uuid::Uuid;

use super::model::Household;
use crate::shared::http_error::is_unique_violation;

const SELECT_COLUMNS: &str = "id, join_code, created_at";

/// How many times `create` retries after colliding with an existing `join_code`. Codes are drawn
/// from 16^8 values, so one retry is already generous — this is just belt and braces.
const JOIN_CODE_ATTEMPTS: u8 = 3;

/// A short code that's quick to read out loud or type into the join field.
fn generate_join_code() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_uppercase()
}

/// Creates a new household with a freshly generated `join_code` and returns it.
pub async fn create(pool: &PgPool) -> Result<Household, sqlx::Error> {
    let mut last_error = None;

    for _ in 0..JOIN_CODE_ATTEMPTS {
        let result = sqlx::query_as::<_, Household>(&format!(
            "INSERT INTO households (join_code) VALUES ($1) RETURNING {SELECT_COLUMNS}"
        ))
        .bind(generate_join_code())
        .fetch_one(pool)
        .await;

        match result {
            Ok(household) => return Ok(household),
            Err(e) if is_unique_violation(&e) => last_error = Some(e),
            Err(e) => return Err(e),
        }
    }

    Err(last_error.expect("loop only exits early on success"))
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

/// Fetches a household by its `join_code`, or `None` if no household uses that code. Used by
/// onboarding when someone joins an existing household.
pub async fn get_by_join_code(
    pool: &PgPool,
    join_code: &str,
) -> Result<Option<Household>, sqlx::Error> {
    sqlx::query_as::<_, Household>(&format!(
        "SELECT {SELECT_COLUMNS} FROM households WHERE join_code = upper($1)"
    ))
    .bind(join_code.trim())
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
