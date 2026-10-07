use std::collections::HashMap;

use sqlx::{PgConnection, PgPool};

use super::model::{
    BulkTransactionPatch, NewTransaction, SortOrder, Transaction, TransactionFilter,
    TransactionPatch, TransactionTag,
};
use crate::features::merchants::repository as merchants_repository;

const SELECT_COLUMNS: &str = "
    t.id, t.household_id, t.household_member_id, t.date, t.original_statement, t.merchant_id,
    mch.name AS merchant_name, mch.logo_url AS merchant_logo_url, t.amount, t.category_id,
    c.name_en AS category_name_en, c.name_fr AS category_name_fr, g.type AS category_type,
    t.account, t.reviewed, t.created_at";

const FROM_JOIN: &str = "FROM transactions t
    JOIN merchants mch ON mch.id = t.merchant_id
    JOIN categories c ON c.id = t.category_id
    JOIN category_groups g ON g.id = c.group_id";

/// Escapes `\`, `%`, and `_` so a search term is matched as a literal substring by `ILIKE ...
/// ESCAPE E'\\'`, rather than having `%`/`_` act as wildcards. Backslashes must be escaped first,
/// or the backslashes introduced for `%`/`_` would themselves get re-escaped.
fn escape_like(term: &str) -> String {
    term.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Fetches every tag attached to any of `transactions` (one batched query, not one per row) and
/// assigns each onto its transaction's `tags` field. A no-op for ids with no tags.
async fn attach_tags(pool: &PgPool, transactions: &mut [Transaction]) -> Result<(), sqlx::Error> {
    if transactions.is_empty() {
        return Ok(());
    }

    let ids: Vec<i64> = transactions.iter().map(|t| t.id).collect();
    let rows: Vec<(i64, i32, String, String)> = sqlx::query_as(
        "SELECT tt.transaction_id, tg.id, tg.name, tg.color
         FROM transaction_tags tt
         JOIN tags tg ON tg.id = tt.tag_id
         WHERE tt.transaction_id = ANY($1)
         ORDER BY tt.transaction_id, tg.id",
    )
    .bind(&ids)
    .fetch_all(pool)
    .await?;

    let mut by_transaction: HashMap<i64, Vec<TransactionTag>> = HashMap::new();
    for (transaction_id, id, name, color) in rows {
        by_transaction
            .entry(transaction_id)
            .or_default()
            .push(TransactionTag { id, name, color });
    }

    for transaction in transactions.iter_mut() {
        if let Some(tags) = by_transaction.remove(&transaction.id) {
            transaction.tags = tags;
        }
    }

    Ok(())
}

/// Replaces a single transaction's tags with exactly `tag_ids`, scoped to `household_id` — ids
/// outside the caller's own household are silently dropped. `tag_ids` being empty just clears the
/// transaction's tags. Takes the transaction connection directly so the caller can run this
/// alongside the transaction row's own insert/update and commit both together.
async fn set_transaction_tags(
    conn: &mut PgConnection,
    household_id: i32,
    transaction_id: i64,
    tag_ids: &[i32],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM transaction_tags WHERE transaction_id = $1")
        .bind(transaction_id)
        .execute(&mut *conn)
        .await?;

    if tag_ids.is_empty() {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO transaction_tags (transaction_id, tag_id)
         SELECT $1, id FROM tags WHERE household_id = $2 AND id = ANY($3)",
    )
    .bind(transaction_id)
    .bind(household_id)
    .bind(tag_ids)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Adds `tag_ids` to every id in `transaction_ids`, scoped to `household_id` — ids outside the
/// caller's own household (on either side) are silently dropped. Never removes a tag a transaction
/// already has (see `BulkTransactionPatch::tag_ids`).
async fn add_transaction_tags(
    conn: &mut PgConnection,
    household_id: i32,
    transaction_ids: &[i64],
    tag_ids: &[i32],
) -> Result<(), sqlx::Error> {
    if transaction_ids.is_empty() || tag_ids.is_empty() {
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO transaction_tags (transaction_id, tag_id)
         SELECT tid, tg.id
         FROM UNNEST($1::bigint[]) AS tid
         CROSS JOIN tags tg
         WHERE tg.household_id = $2 AND tg.id = ANY($3)
         ON CONFLICT (transaction_id, tag_id) DO NOTHING",
    )
    .bind(transaction_ids)
    .bind(household_id)
    .bind(tag_ids)
    .execute(&mut *conn)
    .await?;

    Ok(())
}

/// Inserts a new transaction — scoped to `household_id` and attributed to `household_member_id` —
/// and returns the created row, joined with its merchant, category, and (if
/// `new_transaction.tag_ids` was given) its tags.
///
/// Resolves the merchant per `NewTransaction`'s doc comment: an explicit `merchant_id` is validated
/// against the caller's household (`Err(sqlx::Error::RowNotFound)` if it isn't visible — the
/// handler maps this to a 400, distinct from a `category_id` foreign-key violation); otherwise
/// `merchants::repository::resolve_or_create` matches-or-creates one from `original_statement`.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    household_member_id: i32,
    new_transaction: &NewTransaction,
) -> Result<Transaction, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let merchant_id = match new_transaction.merchant_id {
        Some(id) => {
            if !merchants_repository::is_visible_to_household(pool, household_id, id).await? {
                return Err(sqlx::Error::RowNotFound);
            }
            id
        }
        None => {
            let statement = new_transaction.original_statement.as_deref().unwrap_or("");
            merchants_repository::resolve_or_create(&mut tx, household_id, statement).await?
        }
    };

    // Only reached when original_statement was omitted, which only happens when merchant_id was
    // given explicitly (the handler requires at least one) — so merchant_id above is already a
    // validated, existing id here.
    let original_statement = match &new_transaction.original_statement {
        Some(statement) => statement.clone(),
        None => merchants_repository::get(pool, household_id, merchant_id)
            .await?
            .map(|m| m.name)
            .unwrap_or_default(),
    };

    let mut transaction = sqlx::query_as::<_, Transaction>(&format!(
        "WITH inserted AS (
            INSERT INTO transactions
                (household_id, household_member_id, date, original_statement, merchant_id, amount,
                 category_id, account, reviewed)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM inserted t
         JOIN merchants mch ON mch.id = t.merchant_id
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id"
    ))
    .bind(household_id)
    .bind(household_member_id)
    .bind(new_transaction.date)
    .bind(&original_statement)
    .bind(merchant_id)
    .bind(new_transaction.amount)
    .bind(new_transaction.category_id)
    .bind(&new_transaction.account)
    .bind(new_transaction.reviewed.unwrap_or(true))
    .fetch_one(&mut *tx)
    .await?;

    if let Some(tag_ids) = &new_transaction.tag_ids {
        set_transaction_tags(&mut tx, household_id, transaction.id, tag_ids).await?;
    }

    tx.commit().await?;

    attach_tags(pool, std::slice::from_mut(&mut transaction)).await?;
    Ok(transaction)
}

/// Lists a household's transactions, optionally narrowed by an exact date match, a
/// `start_date`/`end_date` range, an exact `merchant_id`, and/or a free-text search across merchant
/// name, original statement, category name, tag name, and amount. Sorted per `filter.order` (most
/// recent first by default).
pub async fn list(
    pool: &PgPool,
    household_id: i32,
    filter: &TransactionFilter,
) -> Result<Vec<Transaction>, sqlx::Error> {
    // `id DESC` is a stable tie-breaker for rows sharing a date/amount; never built from user input.
    let order_by = match filter.order {
        None | Some(SortOrder::Date) => "t.date DESC, t.id DESC",
        Some(SortOrder::InverseDate) => "t.date ASC, t.id DESC",
        Some(SortOrder::Amount) => "t.amount DESC, t.id DESC",
        Some(SortOrder::InverseAmount) => "t.amount ASC, t.id DESC",
    };

    let escaped_search = filter.search.as_deref().map(escape_like);

    let mut transactions = sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOIN}
         WHERE t.household_id = $1
           AND ($2::date IS NULL OR t.date = $2)
           AND ($3::integer IS NULL OR t.merchant_id = $3)
           AND ($4::text IS NULL OR (
                 mch.name ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR t.original_statement ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR c.name_en ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR c.name_fr ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR t.amount::text ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR EXISTS (
                   SELECT 1 FROM transaction_tags tt
                   JOIN tags tg ON tg.id = tt.tag_id
                   WHERE tt.transaction_id = t.id
                     AND tg.name ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
                 )
           ))
           AND ($5::date IS NULL OR t.date >= $5)
           AND ($6::date IS NULL OR t.date <= $6)
         ORDER BY {order_by}"
    ))
    .bind(household_id)
    .bind(filter.date)
    .bind(filter.merchant_id)
    .bind(&escaped_search)
    .bind(filter.start_date)
    .bind(filter.end_date)
    .fetch_all(pool)
    .await?;

    attach_tags(pool, &mut transactions).await?;
    Ok(transactions)
}

/// Fetches a single transaction by id, scoped to `household_id`, or `None` if it doesn't exist
/// (including when it belongs to a different household).
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i64,
) -> Result<Option<Transaction>, sqlx::Error> {
    let mut transaction = sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOIN} WHERE t.id = $1 AND t.household_id = $2"
    ))
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await?;

    if let Some(transaction) = &mut transaction {
        attach_tags(pool, std::slice::from_mut(transaction)).await?;
    }
    Ok(transaction)
}

/// Applies a partial update (only `Some` fields change), scoped to `household_id`, and returns the
/// updated row (joined with its merchant, category, and tags), or `None` if the id doesn't exist
/// (including when it belongs to a different household). `patch.tag_ids`, if given, *replaces* the
/// full tag set. `patch.merchant_id`, if given, is validated against the caller's household first —
/// `Err(sqlx::Error::RowNotFound)` if it isn't visible, distinct from `Ok(None)` (unknown
/// transaction id), same convention `create` uses.
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i64,
    patch: &TransactionPatch,
) -> Result<Option<Transaction>, sqlx::Error> {
    if let Some(merchant_id) = patch.merchant_id {
        if !merchants_repository::is_visible_to_household(pool, household_id, merchant_id).await? {
            return Err(sqlx::Error::RowNotFound);
        }
    }

    let mut tx = pool.begin().await?;

    let updated = sqlx::query_as::<_, Transaction>(&format!(
        "WITH updated AS (
            UPDATE transactions
            SET date = COALESCE($3, date),
                merchant_id = COALESCE($4, merchant_id),
                amount = COALESCE($5, amount),
                category_id = COALESCE($6, category_id),
                account = COALESCE($7, account)
            WHERE id = $1 AND household_id = $2
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM updated t
         JOIN merchants mch ON mch.id = t.merchant_id
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id"
    ))
    .bind(id)
    .bind(household_id)
    .bind(patch.date)
    .bind(patch.merchant_id)
    .bind(patch.amount)
    .bind(patch.category_id)
    .bind(&patch.account)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(mut transaction) = updated else {
        return Ok(None);
    };

    if let Some(tag_ids) = &patch.tag_ids {
        set_transaction_tags(&mut tx, household_id, transaction.id, tag_ids).await?;
    }

    tx.commit().await?;

    attach_tags(pool, std::slice::from_mut(&mut transaction)).await?;
    Ok(Some(transaction))
}

/// Deletes a transaction by id, scoped to `household_id`. Returns `true` if a row was deleted,
/// `false` if the id didn't exist (including when it belongs to a different household).
pub async fn delete(pool: &PgPool, household_id: i32, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM transactions WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Applies a partial update (only `Some` fields change) to every id in `ids`, scoped to
/// `household_id` — ids that don't exist or belong to a different household are silently skipped.
/// Returns the updated rows (joined with their merchant, category, and tags). `patch.tag_ids`, if
/// given, is *added* to each updated transaction's existing tags (see
/// `BulkTransactionPatch::tag_ids`). `patch.merchant_id`, if given, is validated the same way as
/// `update` before touching any row.
pub async fn bulk_update(
    pool: &PgPool,
    household_id: i32,
    ids: &[i64],
    patch: &BulkTransactionPatch,
) -> Result<Vec<Transaction>, sqlx::Error> {
    if let Some(merchant_id) = patch.merchant_id {
        if !merchants_repository::is_visible_to_household(pool, household_id, merchant_id).await? {
            return Err(sqlx::Error::RowNotFound);
        }
    }

    let mut tx = pool.begin().await?;

    let mut transactions = sqlx::query_as::<_, Transaction>(&format!(
        "WITH updated AS (
            UPDATE transactions
            SET date = COALESCE($3, date),
                merchant_id = COALESCE($4, merchant_id),
                category_id = COALESCE($5, category_id)
            WHERE id = ANY($1) AND household_id = $2
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM updated t
         JOIN merchants mch ON mch.id = t.merchant_id
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id"
    ))
    .bind(ids)
    .bind(household_id)
    .bind(patch.date)
    .bind(patch.merchant_id)
    .bind(patch.category_id)
    .fetch_all(&mut *tx)
    .await?;

    if let Some(tag_ids) = &patch.tag_ids {
        let updated_ids: Vec<i64> = transactions.iter().map(|t| t.id).collect();
        add_transaction_tags(&mut tx, household_id, &updated_ids, tag_ids).await?;
    }

    tx.commit().await?;

    attach_tags(pool, &mut transactions).await?;
    Ok(transactions)
}

/// Deletes every id in `ids`, scoped to `household_id` — ids that don't exist or belong to a
/// different household are silently skipped. Returns the number of rows deleted.
pub async fn bulk_delete(
    pool: &PgPool,
    household_id: i32,
    ids: &[i64],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM transactions WHERE id = ANY($1) AND household_id = $2")
        .bind(ids)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
