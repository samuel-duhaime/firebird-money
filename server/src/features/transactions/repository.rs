use sqlx::PgPool;

use super::model::{NewTransaction, Transaction, TransactionFilter, TransactionPatch};

const SELECT_COLUMNS: &str = "
    t.id, t.date, t.merchant, t.amount, t.category_id,
    c.name_en AS category_name_en, c.name_fr AS category_name_fr, c.type AS category_type,
    t.account, t.created_at";

const FROM_JOIN: &str = "FROM transactions t JOIN categories c ON c.id = t.category_id";

/// Inserts a new transaction and returns the created row, joined with its category.
pub async fn create(pool: &PgPool, new_transaction: &NewTransaction) -> Result<Transaction, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!(
        "WITH inserted AS (
            INSERT INTO transactions (date, merchant, amount, category_id, account)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM inserted t JOIN categories c ON c.id = t.category_id"
    ))
    .bind(new_transaction.date)
    .bind(&new_transaction.merchant)
    .bind(new_transaction.amount)
    .bind(new_transaction.category_id)
    .bind(&new_transaction.account)
    .fetch_one(pool)
    .await
}

/// Lists transactions, optionally narrowed by an exact date match and/or a
/// case-insensitive merchant substring match. Most recent first.
pub async fn list(pool: &PgPool, filter: &TransactionFilter) -> Result<Vec<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOIN}
         WHERE ($1::date IS NULL OR t.date = $1)
           AND ($2::text IS NULL OR t.merchant ILIKE '%' || $2 || '%')
         ORDER BY t.date DESC, t.id DESC"
    ))
    .bind(filter.date)
    .bind(&filter.merchant)
    .fetch_all(pool)
    .await
}

/// Fetches a single transaction by id, or `None` if it doesn't exist.
pub async fn get(pool: &PgPool, id: i64) -> Result<Option<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!(
        "SELECT {SELECT_COLUMNS} {FROM_JOIN} WHERE t.id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update (only `Some` fields change) and returns the updated row (joined with
/// its category), or `None` if the id doesn't exist.
pub async fn update(
    pool: &PgPool,
    id: i64,
    patch: &TransactionPatch,
) -> Result<Option<Transaction>, sqlx::Error> {
    sqlx::query_as::<_, Transaction>(&format!(
        "WITH updated AS (
            UPDATE transactions
            SET date = COALESCE($2, date),
                merchant = COALESCE($3, merchant),
                amount = COALESCE($4, amount),
                category_id = COALESCE($5, category_id),
                account = COALESCE($6, account)
            WHERE id = $1
            RETURNING *
         )
         SELECT {SELECT_COLUMNS}
         FROM updated t JOIN categories c ON c.id = t.category_id"
    ))
    .bind(id)
    .bind(patch.date)
    .bind(&patch.merchant)
    .bind(patch.amount)
    .bind(patch.category_id)
    .bind(&patch.account)
    .fetch_optional(pool)
    .await
}

/// Deletes a transaction by id. Returns `true` if a row was deleted, `false` if the id didn't exist.
pub async fn delete(pool: &PgPool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM transactions WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
