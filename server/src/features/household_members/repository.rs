use sqlx::PgPool;

use super::model::{
    HouseholdMember, HouseholdMemberFilter, HouseholdMemberPatch, NewHouseholdMember,
};

const SELECT_COLUMNS: &str = "id, household_id, user_id, type, created_at";

/// Inserts a new household membership and returns the created row.
pub async fn create(
    pool: &PgPool,
    new_member: &NewHouseholdMember,
) -> Result<HouseholdMember, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "INSERT INTO household_members (household_id, user_id, type)
         VALUES ($1, $2, $3)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(new_member.household_id)
    .bind(new_member.user_id)
    .bind(&new_member.r#type)
    .fetch_one(pool)
    .await
}

/// Lists household memberships, optionally narrowed to a single household and/or user.
pub async fn list(
    pool: &PgPool,
    filter: &HouseholdMemberFilter,
) -> Result<Vec<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "SELECT {SELECT_COLUMNS} FROM household_members
         WHERE ($1::int IS NULL OR household_id = $1)
           AND ($2::int IS NULL OR user_id = $2)
         ORDER BY id"
    ))
    .bind(filter.household_id)
    .bind(filter.user_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single household membership by id, or `None` if it doesn't exist.
pub async fn get(pool: &PgPool, id: i32) -> Result<Option<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "SELECT {SELECT_COLUMNS} FROM household_members WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update (only the role can change) and returns the updated row, or `None` if
/// the id doesn't exist.
pub async fn update(
    pool: &PgPool,
    id: i32,
    patch: &HouseholdMemberPatch,
) -> Result<Option<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "UPDATE household_members
         SET type = COALESCE($2, type)
         WHERE id = $1
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(&patch.r#type)
    .fetch_optional(pool)
    .await
}

/// Deletes a household membership by id. Returns `true` if a row was deleted, `false` if the id
/// didn't exist.
pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM household_members WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
