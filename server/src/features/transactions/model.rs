use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `transactions` table, joined with its category.
#[derive(Debug, Serialize, FromRow)]
pub struct Transaction {
    pub id: i64,
    pub date: NaiveDate,
    pub merchant: String,
    pub amount: Decimal,
    pub category_id: i32,
    pub category_name_en: String,
    pub category_name_fr: String,
    pub category_type: String,
    pub account: String,
    pub created_at: DateTime<Utc>,
}

/// Body for `POST /transactions`. All fields are required; `id` and `created_at` are generated.
#[derive(Debug, Deserialize)]
pub struct NewTransaction {
    pub date: NaiveDate,
    pub merchant: String,
    pub amount: Decimal,
    pub category_id: i32,
    pub account: String,
}

/// Optional query params for `GET /transactions`. Absent fields mean "no filter".
#[derive(Debug, Deserialize)]
pub struct TransactionFilter {
    pub date: Option<NaiveDate>,
    pub merchant: Option<String>,
}

/// Body for `PATCH /transactions/{id}`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct TransactionPatch {
    pub date: Option<NaiveDate>,
    pub merchant: Option<String>,
    pub amount: Option<Decimal>,
    pub category_id: Option<i32>,
    pub account: Option<String>,
}
