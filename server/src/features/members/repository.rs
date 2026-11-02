use sqlx::PgPool;

use super::model::{Member, MemberPatch, MemberPicture};
use crate::features::users::model::picture_url_column;

const SELECT_COLUMNS: &str = concat!(
    "id, household_id, household_role AS role, email, first_name, last_name, status, created_at, ",
    picture_url_column!()
);

/// Lists a household's members.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(&format!(
        "SELECT {SELECT_COLUMNS} FROM users WHERE household_id = $1 ORDER BY id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single member by user id, scoped to `household_id`, or `None` if they don't exist
/// (including when they belong to a different household).
pub async fn get(pool: &PgPool, household_id: i32, id: i32) -> Result<Option<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(&format!(
        "SELECT {SELECT_COLUMNS} FROM users WHERE id = $1 AND household_id = $2"
    ))
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update (only the role can change), scoped to `household_id`, and returns the
/// updated member, or `None` if the id doesn't exist (including when they belong to a different
/// household).
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    patch: &MemberPatch,
) -> Result<Option<Member>, sqlx::Error> {
    sqlx::query_as::<_, Member>(&format!(
        "UPDATE users
         SET household_role = COALESCE($3, household_role)
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(&patch.role)
    .fetch_optional(pool)
    .await
}

/// Removes a member from `household_id` (the user itself is kept, with no household). Returns
/// `true` if they were removed, `false` if the id didn't exist (including when they belong to a
/// different household). Fails with a foreign-key violation while they still have transactions in
/// the household.
pub async fn remove(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE users SET household_id = NULL, household_role = NULL
         WHERE id = $1 AND household_id = $2",
    )
    .bind(id)
    .bind(household_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Fetches a member's picture, scoped to `household_id`, or `None` if they have none (or don't
/// exist, including when they belong to a different household).
pub async fn get_picture(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<MemberPicture>, sqlx::Error> {
    sqlx::query_as::<_, MemberPicture>(
        "SELECT user_pictures.content_type, user_pictures.data
         FROM user_pictures
         JOIN users ON users.id = user_pictures.user_id
         WHERE user_pictures.user_id = $1 AND users.household_id = $2",
    )
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await
}

/// Sets or replaces a member's picture, scoped to `household_id`. Returns `false` if the id doesn't
/// exist (including when they belong to a different household).
pub async fn set_picture(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    content_type: &str,
    data: &[u8],
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO user_pictures (user_id, content_type, data)
         SELECT id, $3, $4 FROM users WHERE id = $1 AND household_id = $2
         ON CONFLICT (user_id) DO UPDATE
         SET content_type = EXCLUDED.content_type, data = EXCLUDED.data, updated_at = now()",
    )
    .bind(id)
    .bind(household_id)
    .bind(content_type)
    .bind(data)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

/// Removes a member's picture if they have one, scoped to `household_id`.
pub async fn delete_picture(pool: &PgPool, household_id: i32, id: i32) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM user_pictures
         USING users
         WHERE users.id = user_pictures.user_id
           AND user_pictures.user_id = $1 AND users.household_id = $2",
    )
    .bind(id)
    .bind(household_id)
    .execute(pool)
    .await?;
    Ok(())
}
