use sqlx::PgPool;

use super::model::{
    HouseholdMember, HouseholdMemberFilter, HouseholdMemberPatch, NewHouseholdMember,
};

const SELECT_COLUMNS: &str = "id, household_id, user_id, type, created_at";

/// Inserts a new household membership and returns the created row. Generic over `PgExecutor` so
/// it can run inside an existing transaction (see `households::repository::create_with_manager`)
/// as well as directly against the pool.
pub async fn create<'e, E>(
    executor: E,
    household_id: i32,
    new_member: &NewHouseholdMember,
) -> Result<HouseholdMember, sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "INSERT INTO household_members (household_id, user_id, type)
         VALUES ($1, $2, $3)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(household_id)
    .bind(new_member.user_id)
    .bind(&new_member.r#type)
    .fetch_one(executor)
    .await
}

/// Lists a household's memberships, optionally narrowed to a single user.
pub async fn list(
    pool: &PgPool,
    household_id: i32,
    filter: &HouseholdMemberFilter,
) -> Result<Vec<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "SELECT {SELECT_COLUMNS} FROM household_members
         WHERE household_id = $1
           AND ($2::int IS NULL OR user_id = $2)
         ORDER BY id"
    ))
    .bind(household_id)
    .bind(filter.user_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single household membership by id, scoped to `household_id`, or `None` if it doesn't
/// exist (including when it belongs to a different household).
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "SELECT {SELECT_COLUMNS} FROM household_members WHERE id = $1 AND household_id = $2"
    ))
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update (only the role can change), scoped to `household_id`, and returns the
/// updated row, or `None` if the id doesn't exist (including when it belongs to a different
/// household).
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    patch: &HouseholdMemberPatch,
) -> Result<Option<HouseholdMember>, sqlx::Error> {
    sqlx::query_as::<_, HouseholdMember>(&format!(
        "UPDATE household_members
         SET type = COALESCE($3, type)
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(&patch.r#type)
    .fetch_optional(pool)
    .await
}

/// Deletes a household membership by id, scoped to `household_id`. Returns `true` if a row was
/// deleted, `false` if the id didn't exist (including when it belongs to a different household).
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM household_members WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
