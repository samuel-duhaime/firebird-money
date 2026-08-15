use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::model::Membership;
use crate::features::users::model::User;

const USER_COLUMNS: &str = "users.id, users.email, users.google_id, users.status, \
     users.first_name, users.last_name, users.avatar_url, users.created_at";

/// Stores a freshly issued magic-link token (by hash) for this user.
pub async fn create_login_token(
    pool: &PgPool,
    user_id: i32,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO login_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(())
}

/// Spends a magic-link token, returning the user it belongs to.
///
/// The `used_at IS NULL` check and the write that fills it happen in one statement, so two
/// simultaneous clicks on the same link can't both succeed. `None` means the token is unknown,
/// already spent, or expired — the caller can't tell which, and shouldn't say.
pub async fn consume_login_token(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<i32>, sqlx::Error> {
    let user_id: Option<(i32,)> = sqlx::query_as(
        "UPDATE login_tokens
         SET used_at = now()
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
         RETURNING user_id",
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?;

    Ok(user_id.map(|(id,)| id))
}

/// Opens a session for this user.
pub async fn create_session(
    pool: &PgPool,
    user_id: i32,
    token_hash: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(pool)
        .await?;
    Ok(())
}

/// Resolves a session cookie to its user, or `None` if the session is unknown, expired, or its
/// user has since been suspended — a suspension takes effect immediately rather than waiting out
/// the session's remaining `SESSION_TTL_DAYS`.
pub async fn get_session_user(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS}
         FROM sessions
         JOIN users ON users.id = sessions.user_id
         WHERE sessions.token_hash = $1
           AND sessions.expires_at > now()
           AND users.status <> 'suspended'"
    ))
    .bind(token_hash)
    .fetch_optional(pool)
    .await
}

/// Ends a session. Returns `true` if one was actually deleted.
pub async fn delete_session(pool: &PgPool, token_hash: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Marks a user as having proven they own their email address.
pub async fn mark_user_verified(pool: &PgPool, user_id: i32) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET status = 'verified' WHERE id = $1 AND status = 'pending'")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Lists every household this user belongs to, with the role they hold in each.
pub async fn list_memberships(pool: &PgPool, user_id: i32) -> Result<Vec<Membership>, sqlx::Error> {
    sqlx::query_as::<_, Membership>(
        "SELECT household_members.household_id, households.join_code, household_members.type
         FROM household_members
         JOIN households ON households.id = household_members.household_id
         WHERE household_members.user_id = $1
         ORDER BY household_members.household_id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}
