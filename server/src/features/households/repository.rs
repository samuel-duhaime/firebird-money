use sqlx::PgPool;
use uuid::Uuid;

use super::model::Household;
use crate::features::category_groups::repository as category_groups_repository;
use crate::features::members::repository as members_repository;
use crate::features::tags::repository as tags_repository;
use crate::features::users::repository as users_repository;
use crate::shared::http_error::is_unique_violation;

const SELECT_COLUMNS: &str = "id, join_code, created_at";

/// How many times `create` retries after colliding with an existing `join_code`. Codes are drawn
/// from 16^8 values, so one retry is already generous — this is just belt and braces.
pub(crate) const JOIN_CODE_ATTEMPTS: u8 = 3;

/// A short code that's quick to read out loud or type into the join field.
fn generate_join_code() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_uppercase()
}

/// Creates a new household with a freshly generated `join_code`, seeds its starter category
/// groups, categories, and tags, and returns the household. Everything happens in one
/// transaction, so a household is never left half-seeded if a later step fails.
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
        tags_repository::seed_defaults(&mut tx, household.id).await?;
        tx.commit().await?;
        return Ok(household);
    }

    Err(last_error.expect("loop only exits early on success"))
}

/// Like `create`, but also makes `user_id` the new household's `family_manager`, in the same
/// transaction as the household's creation and starter-data seeding — used by
/// `POST /auth/onboarding`'s no-`join_code` branch, so a failure at any step never leaves an
/// orphaned, unowned household behind. `member_tag_name` names the manager's own starter tag (see
/// `tags::defaults::member_tag_name`). `Ok(None)` (and nothing created) if the user already
/// belongs to a household, including when a concurrent onboarding request on the same account just
/// put them in one.
pub async fn create_with_manager(
    pool: &PgPool,
    user_id: i32,
    member_tag_name: &str,
) -> Result<Option<Household>, sqlx::Error> {
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
        tags_repository::seed_defaults(&mut tx, household.id).await?;

        if !users_repository::join_household(&mut *tx, user_id, household.id, "family_manager")
            .await?
        {
            return Ok(None);
        }
        tags_repository::add_member_tag(&mut tx, household.id, member_tag_name).await?;

        tx.commit().await?;
        return Ok(Some(household));
    }

    Err(last_error.expect("loop only exits early on success"))
}

/// What joining a household by its code came to, when it didn't fail outright.
#[derive(Debug, PartialEq, Eq)]
pub enum JoinOutcome {
    Joined,
    /// No household uses that code (anymore).
    CodeNotFound,
    AlreadyInHousehold,
}

/// Puts `user_id` into the household behind `join_code` as a `family_member` and adds their own
/// tag, named `member_tag_name` (see `tags::defaults::member_tag_name`), in one transaction — used
/// by `POST /auth/onboarding`'s `join_code` branch. The code is looked up while locking the
/// household, so a code rotated by a removal can't still let someone in.
pub async fn join_by_code(
    pool: &PgPool,
    user_id: i32,
    join_code: &str,
    member_tag_name: &str,
) -> Result<JoinOutcome, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let household_id: Option<i32> =
        sqlx::query_scalar("SELECT id FROM households WHERE join_code = upper($1) FOR UPDATE")
            .bind(join_code.trim())
            .fetch_optional(&mut *tx)
            .await?;
    let Some(household_id) = household_id else {
        return Ok(JoinOutcome::CodeNotFound);
    };

    if !users_repository::join_household(&mut *tx, user_id, household_id, "family_member").await? {
        return Ok(JoinOutcome::AlreadyInHousehold);
    }
    tags_repository::add_member_tag(&mut tx, household_id, member_tag_name).await?;

    tx.commit().await?;
    Ok(JoinOutcome::Joined)
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

/// Gives `household_id` a new `join_code` and returns the household, or `None` if it doesn't
/// exist. One attempt: a collision (one in 16^8) fails with a unique violation. Generic over
/// `PgExecutor` so a removal can rotate it in its own transaction.
pub async fn rotate_join_code<'e, E>(
    executor: E,
    household_id: i32,
) -> Result<Option<Household>, sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_as::<_, Household>(&format!(
        "UPDATE households SET join_code = $2 WHERE id = $1 RETURNING {SELECT_COLUMNS}"
    ))
    .bind(household_id)
    .bind(generate_join_code())
    .fetch_optional(executor)
    .await
}

/// Like `rotate_join_code`, retrying on a collision with another household's code.
pub async fn regenerate_join_code(
    pool: &PgPool,
    household_id: i32,
) -> Result<Option<Household>, sqlx::Error> {
    let mut last_error = None;

    for _ in 0..JOIN_CODE_ATTEMPTS {
        match rotate_join_code(pool, household_id).await {
            Err(e) if is_unique_violation(&e) => last_error = Some(e),
            result => return result,
        }
    }

    Err(last_error.expect("loop only exits early on success"))
}

/// What deleting a household came to, when it didn't fail outright.
#[derive(Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    NotFound,
    /// Someone other than the caller still belongs to it.
    HasOtherMembers,
}

/// Deletes `id` and everything in it (rules, transactions, accounts, institutions, merchants, tags,
/// categories, and category groups), when `user_id` is its only member, who's left with no
/// household. All in one transaction, which holds the household's lock so nobody joins halfway.
pub async fn delete_with_data(
    pool: &PgPool,
    id: i32,
    user_id: i32,
) -> Result<DeleteOutcome, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if !members_repository::lock_household(&mut tx, id).await? {
        return Ok(DeleteOutcome::NotFound);
    }

    let member_ids: Vec<i32> = sqlx::query_scalar("SELECT id FROM users WHERE household_id = $1")
        .bind(id)
        .fetch_all(&mut *tx)
        .await?;
    if member_ids != [user_id] {
        return Ok(DeleteOutcome::HasOtherMembers);
    }

    // In foreign-key order. `transaction_tags` goes with its transactions (ON DELETE CASCADE).
    for statement in [
        "DELETE FROM rules WHERE household_id = $1",
        "DELETE FROM transactions WHERE household_id = $1",
        "DELETE FROM accounts WHERE household_id = $1",
        "DELETE FROM institutions WHERE household_id = $1",
        "DELETE FROM merchants WHERE household_id = $1",
        "DELETE FROM tags WHERE household_id = $1",
        "DELETE FROM categories WHERE household_id = $1",
        "DELETE FROM category_groups WHERE household_id = $1",
        "UPDATE users SET household_id = NULL, household_role = NULL WHERE household_id = $1",
        "DELETE FROM households WHERE id = $1",
    ] {
        sqlx::query(statement).bind(id).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(DeleteOutcome::Deleted)
}
