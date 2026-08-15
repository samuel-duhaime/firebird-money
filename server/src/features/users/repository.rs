use sqlx::PgPool;

use super::model::{NewUser, User, UserPatch};
use crate::shared::http_error::is_unique_violation;

const SELECT_COLUMNS: &str =
    "id, email, google_id, status, first_name, last_name, avatar_url, created_at";

/// Inserts a new user and returns the created row. `status` starts at `pending`.
pub async fn create(pool: &PgPool, new_user: &NewUser) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(&format!(
        "INSERT INTO users (email, google_id)
         VALUES ($1, $2)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(&new_user.email)
    .bind(&new_user.google_id)
    .fetch_one(pool)
    .await
}

/// Fetches a user by email (case insensitive), or `None` if nobody uses it.
pub async fn get_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(&format!(
        "SELECT {SELECT_COLUMNS} FROM users WHERE lower(email) = lower($1)"
    ))
    .bind(email)
    .fetch_optional(pool)
    .await
}

/// Returns the user with this email, creating them if this is the first time we see it — the
/// find-or-create behind passwordless sign-in, where asking for a link *is* signing up.
pub async fn find_or_create_by_email(pool: &PgPool, email: &str) -> Result<User, sqlx::Error> {
    if let Some(user) = get_by_email(pool, email).await? {
        return Ok(user);
    }

    let created = create(
        pool,
        &NewUser {
            email: email.to_string(),
            google_id: None,
        },
    )
    .await;

    match created {
        // Someone asked for a link twice in quick succession and the other request won the insert.
        Err(e) if is_unique_violation(&e) => get_by_email(pool, email)
            .await?
            .ok_or(sqlx::Error::RowNotFound),
        other => other,
    }
}

/// Fetches a single user by id, or `None` if it doesn't exist.
pub async fn get(pool: &PgPool, id: i32) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(&format!("SELECT {SELECT_COLUMNS} FROM users WHERE id = $1"))
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Applies a partial update (only `Some` fields change) and returns the updated row, or `None` if
/// the id doesn't exist.
pub async fn update(
    pool: &PgPool,
    id: i32,
    patch: &UserPatch,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(&format!(
        "UPDATE users
         SET email = COALESCE($2, email),
             google_id = COALESCE($3, google_id),
             status = COALESCE($4, status),
             first_name = COALESCE($5, first_name),
             last_name = COALESCE($6, last_name),
             avatar_url = COALESCE($7, avatar_url)
         WHERE id = $1
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(&patch.email)
    .bind(&patch.google_id)
    .bind(&patch.status)
    .bind(&patch.first_name)
    .bind(&patch.last_name)
    .bind(&patch.avatar_url)
    .fetch_optional(pool)
    .await
}

/// Deletes a user by id. Returns `true` if a row was deleted, `false` if the id didn't exist.
pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
