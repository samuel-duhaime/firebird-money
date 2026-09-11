use sqlx::PgPool;
use uuid::Uuid;

use super::model::Household;
use crate::features::category_groups::repository as category_groups_repository;
use crate::features::household_members::model::NewHouseholdMember;
use crate::features::household_members::repository as household_members_repository;
use crate::shared::http_error::is_unique_violation;

const SELECT_COLUMNS: &str = "id, join_code, created_at";

/// How many times `create` retries after colliding with an existing `join_code`. Codes are drawn
/// from 16^8 values, so one retry is already generous — this is just belt and braces.
const JOIN_CODE_ATTEMPTS: u8 = 3;

/// A short code that's quick to read out loud or type into the join field.
fn generate_join_code() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_uppercase()
}

/// Creates a new household with a freshly generated `join_code`, seeds its starter category groups
/// and categories, and returns the household. Everything happens in one transaction, so a
/// household is never left half-seeded if a later step fails.
pub async fn create(pool: &PgPool) -> Result<Household, sqlx::Error> {
    let mut last_error = None;

    for _ in 0..JOIN_CODE_ATTEMPTS {
        let mut tx = pool.begin().await?;

        let result = sqlx::query_as::<_, Household>(&format!(
            "INSERT INTO households (join_code) VALUES ($1) RETURNING {SELECT_COLUMNS}"
        ))
        .bind(generate_join_code())
        .fetch_one(&mut *tx)
        .await;

        let household = match result {
            Ok(household) => household,
            Err(e) if is_unique_violation(&e) => {
                last_error = Some(e);
                continue;
            }
            Err(e) => return Err(e),
        };

        category_groups_repository::seed_defaults(&mut tx, household.id).await?;
        tx.commit().await?;
        return Ok(household);
    }

    Err(last_error.expect("loop only exits early on success"))
}

/// Like `create`, but also connects `user_id` as the new household's `family_manager`, in the
/// same transaction as the household's creation and starter-data seeding — used by
/// `POST /auth/onboarding`'s no-`join_code` branch, so a failure at any step (including a
/// concurrent onboarding request on the same account racing the `household_members.user_id`
/// unique constraint) never leaves an orphaned, unowned household behind.
pub async fn create_with_manager(pool: &PgPool, user_id: i32) -> Result<Household, sqlx::Error> {
    let mut last_error = None;

    for _ in 0..JOIN_CODE_ATTEMPTS {
        let mut tx = pool.begin().await?;

        let result = sqlx::query_as::<_, Household>(&format!(
            "INSERT INTO households (join_code) VALUES ($1) RETURNING {SELECT_COLUMNS}"
        ))
        .bind(generate_join_code())
        .fetch_one(&mut *tx)
        .await;

        let household = match result {
            Ok(household) => household,
            Err(e) if is_unique_violation(&e) => {
                last_error = Some(e);
                continue;
            }
            Err(e) => return Err(e),
        };

        category_groups_repository::seed_defaults(&mut tx, household.id).await?;

        let new_member = NewHouseholdMember {
            user_id,
            r#type: "family_manager".to_string(),
        };
        household_members_repository::create(&mut *tx, household.id, &new_member).await?;

        tx.commit().await?;
        return Ok(household);
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
