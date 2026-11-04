use sqlx::{PgConnection, PgPool};

use super::model::{Account, AccountOwner, AccountPatch, NewAccount};
use crate::features::institutions::repository as institutions_repository;

/// Every column of an `Account`, selected from a row aliased `a` joined to its institution `i`.
const SELECT_COLUMNS: &str = "
    a.id, a.household_id, a.institution_id, i.name AS institution_name, a.name, a.type,
    a.last_four, a.currency, a.owner_user_id, a.created_at,
    (SELECT COUNT(*) FROM transactions
     WHERE account_id = a.id AND household_id = a.household_id) AS transaction_count";

/// Lists a household's accounts, grouped by institution (A-Z), then A-Z by account name within one.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Account>, sqlx::Error> {
    list_on(&mut *pool.acquire().await?, household_id).await
}

/// `list` on a given connection, so it can run inside a database transaction.
async fn list_on(conn: &mut PgConnection, household_id: i32) -> Result<Vec<Account>, sqlx::Error> {
    sqlx::query_as::<_, Account>(&format!(
        "SELECT {SELECT_COLUMNS}
         FROM accounts a
         JOIN institutions i ON i.id = a.institution_id
         WHERE a.household_id = $1
         ORDER BY lower(i.name), i.id, lower(a.name), a.id"
    ))
    .bind(household_id)
    .fetch_all(&mut *conn)
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
/// maps to a 400 (same convention as `transactions::repository::create`'s merchant check). An owner
/// outside the household fails with a foreign-key violation on `accounts_household_owner_fkey`.
/// `new_account` is expected to be already validated/normalized by the handler, `owner` included.
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
            INSERT INTO accounts
                (household_id, institution_id, name, type, last_four, currency, owner_user_id)
            VALUES ($1, $2, $3, $4, $5, COALESCE($6, 'CAD'), $7)
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
    .bind(new_account.owner.and_then(|owner| owner.user_id()))
    .fetch_one(pool)
    .await
}

/// Applies a partial update, scoped to `household_id`. `last_four` follows a three-state rule:
/// `None` leaves it unchanged, `Some("")` clears it, anything else sets it. Returns `Ok(None)` for
/// an unknown id, and `Err(sqlx::Error::RowNotFound)` when `patch.institution_id` isn't visible to
/// the household — same split as `create`. `owner: Some(Owner::Shared)` clears the owner, unlike
/// the other fields where `None` and "clear" can't be told apart; a new owner goes through
/// `set_owners`, so the account's transactions follow it.
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

    let mut tx = pool.begin().await?;

    if let Some(owner) = patch.owner {
        let owners = [AccountOwner {
            account_id: id,
            owner,
        }];
        if !set_owners(&mut tx, household_id, &owners).await? {
            return Ok(None);
        }
    }

    let account = sqlx::query_as::<_, Account>(&format!(
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
    .fetch_optional(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(account)
}

/// Gives each account in `owners` its new owner, scoped to `household_id`, and moves along every
/// transaction of that account that had the old owner. A transaction with another owner was
/// changed on its own, so it keeps it. The one place an account's owner is written after it's
/// created, so the two never drift apart.
///
/// Returns `false`, having changed nothing the caller commits, when an account isn't the
/// household's. An owner outside the household fails with a foreign-key violation on
/// `accounts_household_owner_fkey`. `owners` must not name an account twice.
pub async fn set_owners(
    conn: &mut PgConnection,
    household_id: i32,
    owners: &[AccountOwner],
) -> Result<bool, sqlx::Error> {
    let account_ids: Vec<i32> = owners.iter().map(|o| o.account_id).collect();
    let new_owners: Vec<Option<i32>> = owners.iter().map(|o| o.owner.user_id()).collect();

    // Locked, so a transaction added to one of them meanwhile waits and then takes the new owner.
    let old_owners: Vec<(i32, Option<i32>)> = sqlx::query_as(
        "SELECT id, owner_user_id FROM accounts
         WHERE household_id = $1 AND id = ANY($2)
         FOR UPDATE",
    )
    .bind(household_id)
    .bind(&account_ids)
    .fetch_all(&mut *conn)
    .await?;
    if old_owners.len() != account_ids.len() {
        return Ok(false);
    }

    // Accounts first, so an owner outside the household is reported on the account's foreign key.
    sqlx::query(
        "UPDATE accounts a
         SET owner_user_id = n.owner_user_id
         FROM UNNEST($2::integer[], $3::integer[]) AS n (account_id, owner_user_id)
         WHERE a.household_id = $1 AND a.id = n.account_id",
    )
    .bind(household_id)
    .bind(&account_ids)
    .bind(&new_owners)
    .execute(&mut *conn)
    .await?;

    let (old_ids, old_owner_ids): (Vec<i32>, Vec<Option<i32>>) = old_owners.into_iter().unzip();
    sqlx::query(
        "UPDATE transactions t
         SET owner_user_id = n.owner_user_id
         FROM UNNEST($2::integer[], $3::integer[]) AS n (account_id, owner_user_id),
              UNNEST($4::integer[], $5::integer[]) AS o (account_id, owner_user_id)
         WHERE t.household_id = $1
           AND t.account_id = n.account_id
           AND o.account_id = n.account_id
           AND t.owner_user_id IS NOT DISTINCT FROM o.owner_user_id",
    )
    .bind(household_id)
    .bind(&account_ids)
    .bind(&new_owners)
    .bind(&old_ids)
    .bind(&old_owner_ids)
    .execute(&mut *conn)
    .await?;

    Ok(true)
}

/// `set_owners` on its own, for `PATCH /accounts/owners`: all or nothing, committed only when
/// every account is the household's. Returns every account as of the change, read before the
/// commit so a failed read can't report a saved change as failed; `None` when an account isn't
/// the household's.
pub async fn update_owners(
    pool: &PgPool,
    household_id: i32,
    owners: &[AccountOwner],
) -> Result<Option<Vec<Account>>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if !set_owners(&mut tx, household_id, owners).await? {
        return Ok(None);
    }
    let accounts = list_on(&mut tx, household_id).await?;
    tx.commit().await?;
    Ok(Some(accounts))
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

/// Finds the household's account at `institution_id` that is clearly this one — the same
/// `last_four`, else the same name and type (ignoring case) when either side has no `last_four`
/// (it's optional, so a saved account may lack it) — else creates it. Returns
/// `(id, created)`. Used by statement import, for an account a statement names that's new; one it
/// creates belongs to `owner_user_id`, the person importing, while one it finds keeps its owner.
pub async fn find_or_create(
    conn: &mut PgConnection,
    household_id: i32,
    institution_id: i32,
    name: &str,
    kind: &str,
    last_four: Option<&str>,
    owner_user_id: i32,
) -> Result<(i32, bool), sqlx::Error> {
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM accounts
         WHERE household_id = $1 AND institution_id = $2
           AND (lower(last_four) = lower($5)
                OR ((last_four IS NULL OR $5::text IS NULL)
                    AND lower(name) = lower($3) AND type = $4))
         -- An exact last-digits match beats a name match.
         ORDER BY (lower(last_four) = lower($5)) IS TRUE DESC, id
         LIMIT 1",
    )
    .bind(household_id)
    .bind(institution_id)
    .bind(name)
    .bind(kind)
    .bind(last_four)
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(id) = existing {
        return Ok((id, false));
    }

    let id: i32 = sqlx::query_scalar(
        "INSERT INTO accounts (household_id, institution_id, name, type, last_four, owner_user_id)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(household_id)
    .bind(institution_id)
    .bind(name)
    .bind(kind)
    .bind(last_four)
    .bind(owner_user_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok((id, true))
}
