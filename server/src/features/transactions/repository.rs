use sqlx::PgPool;

use super::model::{NewTransaction, SortOrder, Transaction, TransactionFilter, TransactionPatch};

const SELECT_COLUMNS: &str = "
    t.id, t.household_id, t.household_member_id, t.date, t.merchant, t.amount, t.category_id,
    c.name_en AS category_name_en, c.name_fr AS category_name_fr, g.type AS category_type,
    t.account, t.reviewed, t.created_at";

const FROM_JOIN: &str = "FROM transactions t
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

/// Inserts a new transaction — scoped to `household_id` and attributed to `household_member_id` —
/// and returns the created row, joined with its category.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    household_member_id: i32,
    new_transaction: &NewTransaction,
) -> Result<Transaction, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!(
        "WITH inserted AS (
            INSERT INTO transactions
                (household_id, household_member_id, date, merchant, amount, category_id, account, reviewed)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM inserted t
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id"
    ))
    .bind(household_id)
    .bind(household_member_id)
    .bind(new_transaction.date)
    .bind(&new_transaction.merchant)
    .bind(new_transaction.amount)
    .bind(new_transaction.category_id)
    .bind(&new_transaction.account)
    .bind(new_transaction.reviewed.unwrap_or(true))
    .fetch_one(pool)
    .await
}

/// Lists a household's transactions, optionally narrowed by an exact date match, a
/// `start_date`/`end_date` range, a case-insensitive merchant substring match, and/or a free-text
/// search across merchant, category name, and amount. Sorted per `filter.order` (most recent first
/// by default).
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

    sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOIN}
         WHERE t.household_id = $1
           AND ($2::date IS NULL OR t.date = $2)
           AND ($3::text IS NULL OR t.merchant ILIKE '%' || $3 || '%')
           AND ($4::text IS NULL OR (
                 t.merchant ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR c.name_en ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR c.name_fr ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
              OR t.amount::text ILIKE '%' || $4 || '%' ESCAPE E'\\\\'
           ))
           AND ($5::date IS NULL OR t.date >= $5)
           AND ($6::date IS NULL OR t.date <= $6)
         ORDER BY {order_by}"
    ))
    .bind(household_id)
    .bind(filter.date)
    .bind(&filter.merchant)
    .bind(&escaped_search)
    .bind(filter.start_date)
    .bind(filter.end_date)
    .fetch_all(pool)
    .await
}

/// Fetches a single transaction by id, scoped to `household_id`, or `None` if it doesn't exist
/// (including when it belongs to a different household).
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i64,
) -> Result<Option<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOIN} WHERE t.id = $1 AND t.household_id = $2"
    ))
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update (only `Some` fields change), scoped to `household_id`, and returns the
/// updated row (joined with its category), or `None` if the id doesn't exist (including when it
/// belongs to a different household).
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i64,
    patch: &TransactionPatch,
) -> Result<Option<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!(
        "WITH updated AS (
            UPDATE transactions
            SET date = COALESCE($3, date),
                merchant = COALESCE($4, merchant),
                amount = COALESCE($5, amount),
                category_id = COALESCE($6, category_id),
                account = COALESCE($7, account)
            WHERE id = $1 AND household_id = $2
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM updated t
         JOIN categories c ON c.id = t.category_id
         JOIN category_groups g ON g.id = c.group_id"
    ))
    .bind(id)
    .bind(household_id)
    .bind(patch.date)
    .bind(&patch.merchant)
    .bind(patch.amount)
    .bind(patch.category_id)
    .bind(&patch.account)
    .fetch_optional(pool)
    .await
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
