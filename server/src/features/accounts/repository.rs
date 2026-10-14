use sqlx::PgPool;

use super::model::{Account, AccountPatch, NewAccount};
use crate::features::institutions::repository as institutions_repository;

/// Every column of an `Account`, selected from a row aliased `a` joined to its institution `i`.
const SELECT_COLUMNS: &str = "
    a.id, a.household_id, a.institution_id, i.name AS institution_name, a.name, a.type,
    a.last_four, a.currency, a.created_at,
    (SELECT COUNT(*) FROM transactions
     WHERE account_id = a.id AND household_id = a.household_id) AS transaction_count";

/// Lists a household's accounts, grouped by institution (A-Z), then A-Z by account name within one.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Account>, sqlx::Error> {
    sqlx::query_as::<_, Account>(&format!(
        "SELECT {SELECT_COLUMNS}
         FROM accounts a
         JOIN institutions i ON i.id = a.institution_id
         WHERE a.household_id = $1
         ORDER BY lower(i.name), i.id, lower(a.name), a.id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single account, scoped to `household_id`, or `None` if it doesn't exist (including
/// when it belongs to a different household).
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<Account>, sqlx::Error> {
    sqlx::query_as::<_, Account>(&format!(
        "SELECT {SELECT_COLUMNS}
         FROM accounts a
         JOIN institutions i ON i.id = a.institution_id
         WHERE a.id = $1 AND a.household_id = $2"
    ))
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await
}

/// Inserts a new account owned by `household_id` and returns it. `new_account.institution_id` must
/// be common or this household's own — `Err(sqlx::Error::RowNotFound)` otherwise, which the handler
/// maps to a 400 (same convention as `transactions::repository::create`'s merchant check).
/// `new_account` is expected to be already validated/normalized by the handler.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    new_account: &NewAccount,
) -> Result<Account, sqlx::Error> {
    if !institutions_repository::is_visible_to_household(
        pool,
        household_id,
        new_account.institution_id,
    )
    .await?
    {
        return Err(sqlx::Error::RowNotFound);
    }

    sqlx::query_as::<_, Account>(&format!(
        "WITH inserted AS (
            INSERT INTO accounts (household_id, institution_id, name, type, last_four, currency)
            VALUES ($1, $2, $3, $4, $5, COALESCE($6, 'CAD'))
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM inserted a
         JOIN institutions i ON i.id = a.institution_id"
    ))
    .bind(household_id)
    .bind(new_account.institution_id)
    .bind(&new_account.name)
    .bind(&new_account.r#type)
    .bind(&new_account.last_four)
    .bind(&new_account.currency)
    .fetch_one(pool)
    .await
}

/// Applies a partial update, scoped to `household_id`. `last_four` follows a three-state rule:
/// `None` leaves it unchanged, `Some("")` clears it, anything else sets it. Returns `Ok(None)` for
/// an unknown id, and `Err(sqlx::Error::RowNotFound)` when `patch.institution_id` isn't visible to
/// the household — same split as `create`.
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    patch: &AccountPatch,
) -> Result<Option<Account>, sqlx::Error> {
    if let Some(institution_id) = patch.institution_id {
        if !institutions_repository::is_visible_to_household(pool, household_id, institution_id)
            .await?
        {
            return Err(sqlx::Error::RowNotFound);
        }
    }

    sqlx::query_as::<_, Account>(&format!(
        "WITH updated AS (
            UPDATE accounts
            SET institution_id = COALESCE($3, institution_id),
                name = COALESCE($4, name),
                type = COALESCE($5, type),
                last_four = CASE WHEN $6::text IS NULL THEN last_four ELSE NULLIF($6, '') END,
                currency = COALESCE($7, currency)
            WHERE id = $1 AND household_id = $2
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM updated a
         JOIN institutions i ON i.id = a.institution_id"
    ))
    .bind(id)
    .bind(household_id)
    .bind(patch.institution_id)
    .bind(&patch.name)
    .bind(&patch.r#type)
    .bind(&patch.last_four)
    .bind(&patch.currency)
    .fetch_optional(pool)
    .await
}

/// Deletes an account, scoped to `household_id`. Returns `false` if the id didn't exist (including
/// when it belongs to a different household). Blocked by a foreign-key violation while any
/// transaction is still in it.
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM accounts WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
