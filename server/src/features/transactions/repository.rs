use std::collections::HashMap;

use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{PgConnection, PgPool};

use super::model::{
    BulkTransactionPatch, ImportResult, ImportTransaction, NewTransaction, SortOrder, Transaction,
    TransactionFilter, TransactionPatch, TransactionTag,
};
use crate::features::accounts::repository as accounts_repository;
use crate::features::institutions::repository as institutions_repository;
use crate::features::merchants::repository as merchants_repository;

const SELECT_COLUMNS: &str = "
    t.id, t.household_id, t.user_id, t.date, t.original_statement, t.merchant_id,
    mch.name AS merchant_name, mch.logo_url AS merchant_logo_url, t.amount, t.category_id,
    c.name_en AS category_name_en, c.name_fr AS category_name_fr, g.type AS category_type,
    t.account_id, a.name AS account_name, t.reviewed, t.source, t.created_at";

const FROM_JOIN: &str = "FROM transactions t
    JOIN merchants mch ON mch.id = t.merchant_id
    JOIN categories c ON c.id = t.category_id
    JOIN category_groups g ON g.id = c.group_id
    JOIN accounts a ON a.id = t.account_id";

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

/// Inserts a new transaction — scoped to `household_id` and attributed to `user_id` —
/// and returns the created row, joined with its merchant, category, and (if
/// `new_transaction.tag_ids` was given) its tags.
///
/// Resolves the merchant per `NewTransaction`'s doc comment: an explicit `merchant_id` is looked up
/// against the caller's household (`Err(sqlx::Error::RowNotFound)` if it isn't visible — the
/// handler maps this to a 400, distinct from a `category_id` foreign-key violation); otherwise
/// `merchants::repository::resolve_or_create` matches-or-creates one from `original_statement`. The
/// explicit-id lookup runs on `pool`, before `tx` opens below — same reasoning `update`/`bulk_update`
/// already follow for their own visibility check — so it never ties up a second pool connection
/// while the insert transaction is in flight.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    user_id: i32,
    new_transaction: &NewTransaction,
) -> Result<Transaction, sqlx::Error> {
    let explicit_merchant = match new_transaction.merchant_id {
        Some(id) => Some(
            merchants_repository::get(pool, household_id, id)
                .await?
                .ok_or(sqlx::Error::RowNotFound)?,
        ),
        None => None,
    };

    let mut tx = pool.begin().await?;

    let merchant_id = match &explicit_merchant {
        Some(merchant) => merchant.id,
        None => {
            let statement = new_transaction.original_statement.as_deref().unwrap_or("");
            merchants_repository::resolve_or_create(&mut tx, household_id, statement).await?
        }
    };

    // Only reached when original_statement was omitted, which only happens when merchant_id was
    // given explicitly (the handler requires at least one) — so explicit_merchant is already Some.
    let original_statement = match &new_transaction.original_statement {
        Some(statement) => statement.clone(),
        None => explicit_merchant.map(|m| m.name).unwrap_or_default(),
    };

    let mut transaction = sqlx::query_as::<_, Transaction>(&format!(
        "WITH inserted AS (
            INSERT INTO transactions
                (household_id, user_id, date, original_statement, merchant_id, amount,
                 category_id, account_id, reviewed)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM inserted t
         JOIN merchants mch ON mch.id = t.merchant_id
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id
         JOIN accounts a ON a.id = t.account_id"
    ))
    .bind(household_id)
    .bind(user_id)
    .bind(new_transaction.date)
    .bind(&original_statement)
    .bind(merchant_id)
    .bind(new_transaction.amount)
    .bind(new_transaction.category_id)
    .bind(new_transaction.account_id)
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

/// Adds statement-import lines the user accepted, all or nothing: one database transaction, so a
/// bad id on line 12 doesn't leave lines 1-11 behind. Each row is stored with `source: import` and
/// `reviewed: true` (the user just reviewed it). A `new_merchant_name` reuses a merchant with that
/// exact name if one exists by now, else creates it — see `find_or_create_named`. A `new_account`
/// does the same with its institution and account, so every line naming it lands in one account.
///
/// Every `merchant_id` is checked against the caller's household first, on `pool` before `tx`
/// opens (same reasoning as `create`): `Err(sqlx::Error::RowNotFound)` if any isn't visible. An
/// unknown `category_id`/`account_id` surfaces as the usual foreign-key violation.
pub async fn create_imported(
    pool: &PgPool,
    household_id: i32,
    user_id: i32,
    lines: &[ImportTransaction],
) -> Result<ImportResult, sqlx::Error> {
    let mut explicit_ids: Vec<i32> = lines.iter().filter_map(|l| l.merchant_id).collect();
    explicit_ids.sort_unstable();
    explicit_ids.dedup();
    if !explicit_ids.is_empty() {
        let visible = merchants_repository::visible_ids(pool, household_id, &explicit_ids).await?;
        if visible.len() != explicit_ids.len() {
            return Err(sqlx::Error::RowNotFound);
        }
    }

    let mut tx = pool.begin().await?;
    let mut created_merchant_count = 0;
    let mut created_account_count = 0;

    for line in lines {
        let merchant_id = match (line.merchant_id, line.new_merchant_name.as_deref()) {
            (Some(id), _) => id,
            (None, Some(name)) => {
                let (id, created) =
                    merchants_repository::find_or_create_named(&mut tx, household_id, name.trim())
                        .await?;
                if created {
                    created_merchant_count += 1;
                }
                id
            }
            // The handler rejects a line with neither before getting here.
            (None, None) => return Err(sqlx::Error::RowNotFound),
        };

        let account_id = match (line.account_id, &line.new_account) {
            (Some(id), _) => id,
            (None, Some(account)) => {
                let (institution_id, _) = institutions_repository::find_or_create_named(
                    &mut tx,
                    household_id,
                    account.institution_name.trim(),
                )
                .await?;
                let (id, created) = accounts_repository::find_or_create(
                    &mut tx,
                    household_id,
                    institution_id,
                    account.name.trim(),
                    &account.kind,
                    account.last_four.as_deref(),
                )
                .await?;
                if created {
                    created_account_count += 1;
                }
                id
            }
            // The handler rejects a line with neither before getting here.
            (None, None) => return Err(sqlx::Error::RowNotFound),
        };

        let (transaction_id,): (i64,) = sqlx::query_as(
            "INSERT INTO transactions
                (household_id, user_id, date, original_statement, merchant_id, amount,
                 category_id, account_id, reviewed, source)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, 'import')
             RETURNING id",
        )
        .bind(household_id)
        .bind(user_id)
        .bind(line.date)
        .bind(line.original_statement.trim())
        .bind(merchant_id)
        .bind(line.amount)
        .bind(line.category_id)
        .bind(account_id)
        .fetch_one(&mut *tx)
        .await?;

        if !line.tag_ids.is_empty() {
            set_transaction_tags(&mut tx, household_id, transaction_id, &line.tag_ids).await?;
        }
    }

    tx.commit().await?;
    Ok(ImportResult {
        created_count: lines.len(),
        created_merchant_count,
        created_account_count,
    })
}

/// `(id, date, amount, account_id)` of every one of the household's transactions dated on any of
/// `dates` — what statement import compares freshly read lines against to flag likely duplicates.
pub async fn list_on_dates(
    pool: &PgPool,
    household_id: i32,
    dates: &[NaiveDate],
) -> Result<Vec<(i64, NaiveDate, Decimal, i32)>, sqlx::Error> {
    if dates.is_empty() {
        return Ok(Vec::new());
    }
    sqlx::query_as(
        "SELECT id, date, amount, account_id FROM transactions
         WHERE household_id = $1 AND date = ANY($2)
         ORDER BY id",
    )
    .bind(household_id)
    .bind(dates)
    .fetch_all(pool)
    .await
}

/// Lists a household's transactions, optionally narrowed by an exact date match, a
/// `start_date`/`end_date` range, an exact `merchant_id` or `account_id`, a free-text search across
/// merchant name, original statement, category name, and amount, and/or the Filters popover's
/// multi-select (`category_ids`, `merchant_ids`, `account_ids`, `tag_ids`/`untagged`) and amount
/// filters.
/// Sorted per `filter.order` (most recent first by default).
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
    let [greater_than, less_than, at_least, at_most] = filter.amount_bounds();

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
           ))
           AND ($5::date IS NULL OR t.date >= $5)
           AND ($6::date IS NULL OR t.date <= $6)
           AND ($7::integer IS NULL OR t.account_id = $7)
           AND ($8::integer[] IS NULL OR t.category_id = ANY($8))
           AND ($9::integer[] IS NULL OR t.merchant_id = ANY($9))
           AND ($10::integer[] IS NULL OR t.account_id = ANY($10))
           AND (($11::integer[] IS NULL AND NOT $12)
              OR EXISTS (
                   SELECT 1 FROM transaction_tags tt
                   WHERE tt.transaction_id = t.id AND tt.tag_id = ANY($11)
                 )
              OR ($12 AND NOT EXISTS (
                   SELECT 1 FROM transaction_tags tt WHERE tt.transaction_id = t.id
                 )))
           AND ($13::numeric IS NULL OR t.amount > $13)
           AND ($14::numeric IS NULL OR t.amount < $14)
           AND ($15::numeric IS NULL OR t.amount >= $15)
           AND ($16::numeric IS NULL OR t.amount <= $16)
         ORDER BY {order_by}"
    ))
    .bind(household_id)
    .bind(filter.date)
    .bind(filter.merchant_id)
    .bind(&escaped_search)
    .bind(filter.start_date)
    .bind(filter.end_date)
    .bind(filter.account_id)
    .bind(&filter.category_ids)
    .bind(&filter.merchant_ids)
    .bind(&filter.account_ids)
    .bind(&filter.tag_ids)
    .bind(filter.untagged.unwrap_or(false))
    .bind(greater_than)
    .bind(less_than)
    .bind(at_least)
    .bind(at_most)
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
                account_id = COALESCE($7, account_id)
            WHERE id = $1 AND household_id = $2
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM updated t
         JOIN merchants mch ON mch.id = t.merchant_id
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id
         JOIN accounts a ON a.id = t.account_id"
    ))
    .bind(id)
    .bind(household_id)
    .bind(patch.date)
    .bind(patch.merchant_id)
    .bind(patch.amount)
    .bind(patch.category_id)
    .bind(patch.account_id)
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
                category_id = COALESCE($5, category_id),
                account_id = COALESCE($6, account_id)
            WHERE id = ANY($1) AND household_id = $2
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM updated t
         JOIN merchants mch ON mch.id = t.merchant_id
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id
         JOIN accounts a ON a.id = t.account_id"
    ))
    .bind(ids)
    .bind(household_id)
    .bind(patch.date)
    .bind(patch.merchant_id)
    .bind(patch.category_id)
    .bind(patch.account_id)
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
