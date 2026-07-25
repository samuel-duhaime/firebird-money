use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

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
    pub reviewed: bool,
    pub created_at: DateTime<Utc>,
}

/// Body for `POST /transactions`. `id` and `created_at` are generated. `reviewed` defaults to
/// `true` when absent; automated imports set it to `false` so they can be found later.
#[derive(Debug, Deserialize)]
pub struct NewTransaction {
    pub date: NaiveDate,
    pub merchant: String,
    pub amount: Decimal,
    pub category_id: i32,
    pub account: String,
    pub reviewed: Option<bool>,
}

/// In-memory status of an async budget-file import, tracked for as long as this server process
/// runs — a job doesn't need to survive a restart, since a restart also kills the subprocess
/// tracking it.
#[derive(Debug, Clone, Serialize)]
pub struct ImportJob {
    pub id: Uuid,
    pub status: ImportJobStatus,
    pub file_name: String,
    pub created_count: Option<i32>,
    pub failed_count: Option<i32>,
    pub skipped_count: Option<i32>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Status of an [`ImportJob`]. `Succeeded`/`Failed` are terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportJobStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

/// Body for `PATCH /transactions/import/jobs/{id}` — how the unattended import subprocess itself
/// reports its final result back to the server.
#[derive(Debug, Deserialize)]
pub struct ImportJobReport {
    pub status: ImportJobStatus,
    pub created_count: Option<i32>,
    pub failed_count: Option<i32>,
    pub skipped_count: Option<i32>,
    pub error_message: Option<String>,
}

/// Optional query params for `GET /transactions`. Absent fields mean "no filter".
#[derive(Debug, Deserialize)]
pub struct TransactionFilter {
    pub date: Option<NaiveDate>,
    pub merchant: Option<String>,
    /// Case-insensitive substring match against merchant, category name, or amount.
    pub search: Option<String>,
    /// Sort order. Defaults to `Date` (most recent first) when absent.
    pub order: Option<SortOrder>,
}

/// Sort order for `GET /transactions`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    /// Most recent date first (default).
    Date,
    /// Oldest date first.
    InverseDate,
    /// Highest amount first.
    Amount,
    /// Lowest amount first.
    InverseAmount,
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
