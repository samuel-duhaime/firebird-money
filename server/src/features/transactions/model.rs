use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// A tag attached to a transaction, as embedded in `Transaction::tags` — just enough to render it
/// (name, color), not the full `tags::model::Tag` row (household id, created_at, ... are noise
/// here).
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TransactionTag {
    pub id: i32,
    pub name: String,
    pub color: String,
}

/// A single row in the `transactions` table, joined with its category and its attached tags.
#[derive(Debug, Serialize, FromRow)]
pub struct Transaction {
    pub id: i64,
    pub household_id: i32,
    /// Which household member this transaction is attributed to — the `household_members` row of
    /// whoever created it, not the free-text `account` column (a placeholder for a future *bank*
    /// account concept).
    pub household_member_id: i32,
    pub date: NaiveDate,
    /// The raw payee/merchant text from the bank or import file, e.g. "STARBUCKS STORE #4521" —
    /// kept forever as the immutable historical record of what the source actually said. Never
    /// user-editable after creation; `merchant_id` is what the UI reads/writes.
    pub original_statement: String,
    pub merchant_id: i32,
    pub merchant_name: String,
    pub merchant_logo_url: Option<String>,
    pub amount: Decimal,
    pub category_id: i32,
    pub category_name_en: String,
    pub category_name_fr: String,
    pub category_type: String,
    pub account: String,
    pub reviewed: bool,
    pub created_at: DateTime<Utc>,
    /// Never read directly off a `transactions` row — there's no such column. Always populated
    /// after the fact by `repository::attach_tags`, in a second batched query (not a `JOIN` on the
    /// main query, since a transaction can carry several tags and that would multiply rows).
    #[sqlx(skip)]
    pub tags: Vec<TransactionTag>,
}

/// Body for `POST /transactions`. `id` and `created_at` are generated; `household_id` and
/// `household_member_id` are never read from the body — they're always the caller's own household
/// and membership, from `CurrentUser`. `reviewed` defaults to `true` when absent; automated
/// imports set it to `false` so they can be found later.
///
/// At least one of `merchant_id`/`original_statement` must be given (checked in the handler):
/// - `merchant_id` given, `original_statement` absent: used directly (validated against the
///   caller's household — see `merchants::repository::is_visible_to_household`), `original_statement`
///   defaults to that merchant's own name. The shape a manual "Add Transaction" picker uses.
/// - `original_statement` given, `merchant_id` absent: `merchants::repository::resolve_or_create`
///   matches an existing merchant, or creates a new custom one named exactly `original_statement` if
///   nothing matches. The shape the budget-file importer uses.
/// - Both given: `merchant_id` is used as-is, `original_statement` stored as given (lets an import
///   override the resolved merchant while still keeping the real statement text on record).
#[derive(Debug, Deserialize)]
pub struct NewTransaction {
    pub date: NaiveDate,
    pub original_statement: Option<String>,
    pub merchant_id: Option<i32>,
    pub amount: Decimal,
    pub category_id: i32,
    pub account: String,
    pub reviewed: Option<bool>,
    /// Optional; absent or `[]` means no tags. Ids outside the caller's own household are
    /// silently dropped rather than rejected, same as `BulkUpdateRequest`'s `ids`.
    pub tag_ids: Option<Vec<i32>>,
}

/// In-memory status of an async budget-file import, tracked for as long as this server process
/// runs — a job doesn't need to survive a restart, since a restart also kills the subprocess
/// tracking it.
#[derive(Debug, Clone, Serialize)]
pub struct ImportJob {
    pub id: Uuid,
    /// Whoever kicked off the import — checked on every later read so one household can't poll or
    /// complete another's job by guessing its (random, but not secret-strength) UUID.
    #[serde(skip_serializing)]
    pub household_id: i32,
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
    pub merchant_id: Option<i32>,
    /// Case-insensitive substring match against merchant name, original statement, category name,
    /// tag name, or amount.
    pub search: Option<String>,
    /// Inclusive lower bound on `date`.
    pub start_date: Option<NaiveDate>,
    /// Inclusive upper bound on `date`.
    pub end_date: Option<NaiveDate>,
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

/// Body for `PATCH /transactions/{id}`. `None` fields are left unchanged. There's no
/// `original_statement` here — it's immutable after creation; only `merchant_id` is editable.
#[derive(Debug, Deserialize)]
pub struct TransactionPatch {
    pub date: Option<NaiveDate>,
    pub merchant_id: Option<i32>,
    pub amount: Option<Decimal>,
    pub category_id: Option<i32>,
    pub account: Option<String>,
    /// `None` leaves tags unchanged. `Some(ids)` *replaces* the full tag set with exactly `ids`
    /// (so `Some([])` clears every tag) — unlike `BulkTransactionPatch::tag_ids`, which only adds.
    /// Ids outside the caller's own household are silently dropped.
    pub tag_ids: Option<Vec<i32>>,
}

/// The subset of [`TransactionPatch`] fields the bulk "edit multiple" panel exposes — no `amount`
/// or `account`, since setting one dollar amount or account across several different transactions
/// isn't a sensible bulk operation.
#[derive(Debug, Deserialize)]
pub struct BulkTransactionPatch {
    pub date: Option<NaiveDate>,
    pub merchant_id: Option<i32>,
    pub category_id: Option<i32>,
    /// `None` (or `Some([])`) leaves tags unchanged. `Some(ids)` *adds* `ids` to each selected
    /// transaction's existing tags — unlike `TransactionPatch::tag_ids`, nothing already on a
    /// transaction is ever removed this way. Ids outside the caller's own household are silently
    /// dropped.
    pub tag_ids: Option<Vec<i32>>,
}

/// Body for `PATCH /transactions/bulk`.
#[derive(Debug, Deserialize)]
pub struct BulkUpdateRequest {
    pub ids: Vec<i64>,
    pub patch: BulkTransactionPatch,
}

/// Body for `DELETE /transactions/bulk`.
#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
    pub ids: Vec<i64>,
}
