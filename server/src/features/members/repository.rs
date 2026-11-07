use sqlx::PgPool;

use super::model::{Member, MemberPatch, MemberPicture};
use crate::features::households::repository::{self as households_repository, JOIN_CODE_ATTEMPTS};
use crate::features::users::model::picture_url_column;
use crate::shared::http_error::is_unique_violation;

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

/// What a change to someone's membership came to, when it didn't fail outright.
#[derive(Debug)]
pub enum MemberOutcome<T> {
    Done(T),
    /// No such member (including when they belong to a different household).
    NotFound,
    /// It would leave a household that still has other people in it without a `family_manager`.
    LastManager,
    /// They're the only one left: leaving would orphan the household, so it's deleted instead.
    Alone,
    /// The person asking is no longer a `family_manager` (someone changed their role meanwhile).
    NotManager,
}

/// Locks `household_id` for the rest of `tx`, so changes to its membership (role changes,
/// removals, people joining or leaving) happen one at a time and each checks the result of the
/// last. `FOR UPDATE` rather than `FOR NO KEY UPDATE`: a removal also changes the `join_code`, and
/// taking the strongest lock up front means it never has to upgrade while holding a member's row.
pub(crate) async fn lock_household(
    tx: &mut sqlx::PgConnection,
    household_id: i32,
) -> Result<bool, sqlx::Error> {
    let found: Option<i32> =
        sqlx::query_scalar("SELECT id FROM households WHERE id = $1 FOR UPDATE")
            .bind(household_id)
            .fetch_optional(tx)
            .await?;
    Ok(found.is_some())
}

/// The ids of `household_id`'s `family_manager`s.
async fn manager_ids(
    tx: &mut sqlx::PgConnection,
    household_id: i32,
) -> Result<Vec<i32>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM users WHERE household_id = $1 AND household_role = 'family_manager'",
    )
    .bind(household_id)
    .fetch_all(tx)
    .await
}

/// Applies a partial update (only the role can change), scoped to `household_id`, and returns the
/// updated member. `actor_id` must still be a `family_manager` once the household is locked.
/// Refuses to demote the household's last `family_manager`.
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    actor_id: i32,
    id: i32,
    patch: &MemberPatch,
) -> Result<MemberOutcome<Member>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_household(&mut tx, household_id).await?;

    let managers = manager_ids(&mut tx, household_id).await?;
    if !managers.contains(&actor_id) {
        return Ok(MemberOutcome::NotManager);
    }
    if patch.role.as_deref() == Some("family_member") && managers == [id] {
        return Ok(MemberOutcome::LastManager);
    }

    let member = sqlx::query_as::<_, Member>(&format!(
        "UPDATE users
         SET household_role = COALESCE($3, household_role)
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(&patch.role)
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(member.map_or(MemberOutcome::NotFound, MemberOutcome::Done))
}

/// Takes `id` out of `household_id` (the user itself is kept, with no household): the accounts and
/// transactions they own become Shared, and the reviews waiting on them go back to Anyone. The
/// transactions they created stay in the household, still attributed to them. Returns `false` if
/// they aren't in it.
async fn detach(
    tx: &mut sqlx::PgConnection,
    household_id: i32,
    id: i32,
) -> Result<bool, sqlx::Error> {
    // Locks the member first: giving them an account takes a lock on this row (through the owner
    // foreign key) that this one waits for. Without it, an account given to them after the update
    // below reads `accounts` would fail the removal on its owner foreign key.
    let found: Option<i32> =
        sqlx::query_scalar("SELECT id FROM users WHERE id = $1 AND household_id = $2 FOR UPDATE")
            .bind(id)
            .bind(household_id)
            .fetch_optional(&mut *tx)
            .await?;
    if found.is_none() {
        return Ok(false);
    }

    sqlx::query(
        "UPDATE accounts SET owner_user_id = NULL WHERE owner_user_id = $1 AND household_id = $2",
    )
    .bind(id)
    .bind(household_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE transactions SET owner_user_id = NULL
         WHERE owner_user_id = $1 AND household_id = $2",
    )
    .bind(id)
    .bind(household_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE transactions SET review_assignee_user_id = NULL
         WHERE review_assignee_user_id = $1 AND household_id = $2",
    )
    .bind(id)
    .bind(household_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "UPDATE users SET household_id = NULL, household_role = NULL
         WHERE id = $1 AND household_id = $2",
    )
    .bind(id)
    .bind(household_id)
    .execute(&mut *tx)
    .await?;

    Ok(true)
}

/// Removes someone else from `household_id` (see `detach`) and gives the household a new
/// `join_code`, since they still know the old one. `actor_id` must still be a `family_manager`
/// once the household is locked.
pub async fn remove(
    pool: &PgPool,
    household_id: i32,
    actor_id: i32,
    id: i32,
) -> Result<MemberOutcome<()>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_household(&mut tx, household_id).await?;

    if !manager_ids(&mut tx, household_id)
        .await?
        .contains(&actor_id)
    {
        return Ok(MemberOutcome::NotManager);
    }
    if !detach(&mut tx, household_id, id).await? {
        return Ok(MemberOutcome::NotFound);
    }

    // Each attempt in its own savepoint, so colliding with another household's code (one in 16^8)
    // doesn't abort the removal.
    let mut attempts = 0;
    loop {
        let mut savepoint = sqlx::Connection::begin(&mut *tx).await?;
        match households_repository::rotate_join_code(&mut *savepoint, household_id).await {
            Ok(_) => {
                savepoint.commit().await?;
                break;
            }
            Err(e) if is_unique_violation(&e) && attempts + 1 < JOIN_CODE_ATTEMPTS => {
                savepoint.rollback().await?;
                attempts += 1;
            }
            Err(e) => return Err(e),
        }
    }

    tx.commit().await?;
    Ok(MemberOutcome::Done(()))
}

/// Takes `id` out of their own household `household_id` (see `detach`). Refused for the last
/// `family_manager` while others remain, and for the only member, who deletes the household
/// instead. The `join_code` stays: they left on their own.
pub async fn leave(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<MemberOutcome<()>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_household(&mut tx, household_id).await?;

    let member_ids: Vec<i32> = sqlx::query_scalar("SELECT id FROM users WHERE household_id = $1")
        .bind(household_id)
        .fetch_all(&mut *tx)
        .await?;
    if !member_ids.contains(&id) {
        return Ok(MemberOutcome::NotFound);
    }
    if member_ids.len() == 1 {
        return Ok(MemberOutcome::Alone);
    }
    if manager_ids(&mut tx, household_id).await? == [id] {
        return Ok(MemberOutcome::LastManager);
    }

    detach(&mut tx, household_id, id).await?;
    tx.commit().await?;
    Ok(MemberOutcome::Done(()))
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
/// exist (including when they belong to a different household). `FOR SHARE` makes it wait for any
/// household change in progress on that member and check against the result.
pub async fn set_picture(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    content_type: &str,
    data: &[u8],
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO user_pictures (user_id, content_type, data)
         SELECT id, $3, $4 FROM users WHERE id = $1 AND household_id = $2 FOR SHARE
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
