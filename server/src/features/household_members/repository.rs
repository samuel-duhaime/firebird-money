use sqlx::PgPool;

use super::model::{HouseholdMember, HouseholdMemberPatch};

const SELECT_COLUMNS: &str = "id, household_id, household_role AS role, email, first_name, \
     last_name, status, created_at";

/// Lists a household's members.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "SELECT {SELECT_COLUMNS} FROM users WHERE household_id = $1 ORDER BY id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single member by user id, scoped to `household_id`, or `None` if they don't exist
/// (including when they belong to a different household).
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
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
    patch: &HouseholdMemberPatch,
) -> Result<Option<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
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
