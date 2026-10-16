use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
    /// Which household member this transaction is attributed to — the user who created it. Not to
    /// be confused with `account_id`, the bank account it's in.
    pub user_id: i32,
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
    /// The household's account (see `accounts`) this transaction is in.
    pub account_id: i32,
    pub account_name: String,
    pub reviewed: bool,
    pub source: TransactionSource,
    pub created_at: DateTime<Utc>,
    /// Never read directly off a `transactions` row — there's no such column. Always populated
    /// after the fact by `repository::attach_tags`, in a second batched query (not a `JOIN` on the
    /// main query, since a transaction can carry several tags and that would multiply rows).
    #[sqlx(skip)]
    pub tags: Vec<TransactionTag>,
}

/// Body for `POST /transactions`. `id` and `created_at` are generated; `household_id` and
/// `user_id` are never read from the body — they're always the caller's own household and user,
/// from `CurrentUser`. `reviewed` defaults to `true` when absent. Always stored with
/// `source: manual` — statement imports go through `POST /transactions/import` instead.
///
/// At least one of `merchant_id`/`original_statement` must be given (checked in the handler):
/// - `merchant_id` given, `original_statement` absent: used directly (validated against the
///   caller's household — see `merchants::repository::is_visible_to_household`), `original_statement`
///   defaults to that merchant's own name. The shape a manual "Add Transaction" picker uses.
/// - `original_statement` given, `merchant_id` absent: `merchants::repository::resolve_or_create`
///   matches an existing merchant, or creates a new custom one named exactly `original_statement` if
///   nothing matches. For a caller that only has raw statement text.
/// - Both given: `merchant_id` is used as-is, `original_statement` stored as given (overrides the
///   resolved merchant while still keeping the real statement text on record).
#[derive(Debug, Deserialize)]
pub struct NewTransaction {
    pub date: NaiveDate,
    pub original_statement: Option<String>,
    pub merchant_id: Option<i32>,
    pub amount: Decimal,
    pub category_id: i32,
    /// Must be one of the caller's own household's accounts (enforced by the composite
    /// `transactions_household_account_fkey`).
    pub account_id: i32,
    pub reviewed: Option<bool>,
    /// Optional; absent or `[]` means no tags. Ids outside the caller's own household are
    /// silently dropped rather than rejected, same as `BulkUpdateRequest`'s `ids`.
    pub tag_ids: Option<Vec<i32>>,
}

/// Where a transaction came from. Set once on insert, never changed afterward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TransactionSource {
    /// Typed in by hand (`POST /transactions`).
    Manual,
    /// Read from a bank statement and accepted by the user (`POST /transactions/import`).
    Import,
}

/// One transaction read from a bank statement by `POST /transactions/import/read`, already matched
/// against the caller's household — every id here is one of the household's own (or a common
/// merchant). Nothing is stored yet: the client shows these for review, then sends back the ones
/// the user accepts to `POST /transactions/import` as [`ImportTransaction`]s.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatementTransaction {
    pub date: NaiveDate,
    /// The line's description exactly as the statement printed it.
    pub original_statement: String,
    /// Always positive, same convention as every other transaction: direction comes from the
    /// category, not the sign.
    pub amount: Decimal,
    /// An existing merchant (common or the household's own). Exactly one of `merchant_id` and
    /// `new_merchant_name` is set.
    pub merchant_id: Option<i32>,
    /// A merchant the household doesn't have yet, created only if the user accepts the line.
    pub new_merchant_name: Option<String>,
    /// `None` only when the household has no category to fall back on — the user must pick one.
    pub category_id: Option<i32>,
    /// `false` when the category is a guess the user should double-check.
    pub category_confident: bool,
    /// `None` when neither the request nor the statement matched one of the household's accounts.
    pub account_id: Option<i32>,
    /// The account the statement names when it isn't one of the household's yet — created only if
    /// the user accepts the line with it. Only set when `account_id` is `None`.
    pub new_account: Option<SuggestedAccount>,
    pub tag_ids: Vec<i32>,
    /// An existing transaction with the same date, amount, and account (when known) — most likely
    /// this same line, imported or typed in before.
    pub duplicate_of_transaction_id: Option<i64>,
}

/// An account a statement names that the household doesn't have yet. Statement import suggests it
/// on the lines that belong to it; `POST /transactions/import` creates it (and its institution,
/// when that's new too) on first use, reusing it for every other line naming the same account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestedAccount {
    /// e.g. "Desjardins". Matched against existing institutions by name or alias first.
    pub institution_name: String,
    /// What to call it, e.g. "Visa" or "Compte conjoint".
    pub name: String,
    /// One of `accounts::model::ACCOUNT_TYPES`.
    #[serde(rename = "type")]
    pub kind: String,
    pub last_four: Option<String>,
}

/// One line of `POST /transactions/import/read`'s streamed response (newline-delimited JSON), so
/// the client can show real progress while the statement is read: `reading`, any number of
/// `found`, `matching`, `checking_duplicates`, then exactly one `done` or `error`.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StatementReadEvent {
    /// Claude has started reading the statement.
    Reading,
    /// How many lines Claude has found so far — sent each time it goes up.
    Found { count: usize },
    /// Matching the lines to the household's merchants, categories, accounts, and tags.
    Matching,
    /// Comparing the lines with transactions already added.
    CheckingDuplicates,
    /// The statement's lines, ready to review. Nothing is stored.
    Done {
        transactions: Vec<StatementTransaction>,
    },
    /// Reading failed. `error` is a localized message to show as-is.
    Error { error: String },
}

/// One accepted line in `POST /transactions/import`. Same shape as [`StatementTransaction`] minus
/// the review-only hints, with the fields the user had to settle (category, account) now required.
#[derive(Debug, Deserialize)]
pub struct ImportTransaction {
    pub date: NaiveDate,
    pub original_statement: String,
    pub amount: Decimal,
    /// Exactly one of `merchant_id` and `new_merchant_name` must be set (checked in the handler).
    pub merchant_id: Option<i32>,
    pub new_merchant_name: Option<String>,
    pub category_id: i32,
    /// Exactly one of `account_id` and `new_account` must be set (checked in the handler).
    pub account_id: Option<i32>,
    pub new_account: Option<SuggestedAccount>,
    /// Ids outside the caller's own household are silently dropped, same as `NewTransaction`.
    #[serde(default)]
    pub tag_ids: Vec<i32>,
}

/// Body for `POST /transactions/import`.
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    pub transactions: Vec<ImportTransaction>,
}

/// Response for `POST /transactions/import`.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct ImportResult {
    pub created_count: usize,
    /// How many of `new_merchant_name`s didn't exist yet and were created.
    pub created_merchant_count: usize,
    /// How many distinct `new_account`s didn't exist yet and were created.
    pub created_account_count: usize,
}

/// Optional query params for `GET /transactions`. Absent fields mean "no filter".
#[derive(Debug, Deserialize)]
pub struct TransactionFilter {
    pub date: Option<NaiveDate>,
    pub merchant_id: Option<i32>,
    pub account_id: Option<i32>,
    /// Case-insensitive substring match against merchant name, original statement, category name,
    /// or amount.
    pub search: Option<String>,
    /// Inclusive lower bound on `date`.
    pub start_date: Option<NaiveDate>,
    /// Inclusive upper bound on `date`.
    pub end_date: Option<NaiveDate>,
    /// Sort order. Defaults to `Date` (most recent first) when absent.
    pub order: Option<SortOrder>,
    /// Comma-separated category ids, e.g. `3,7` — matches transactions in any of them.
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub category_ids: Option<Vec<i32>>,
    /// Comma-separated merchant ids — matches transactions with any of them.
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub merchant_ids: Option<Vec<i32>>,
    /// Comma-separated account ids — matches transactions in any of them.
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub account_ids: Option<Vec<i32>>,
    /// Comma-separated tag ids — matches transactions carrying at least one of them.
    #[serde(default, deserialize_with = "deserialize_id_list")]
    pub tag_ids: Option<Vec<i32>>,
    /// `true` also matches transactions without any tag (OR-ed with `tag_ids` when both are set).
    pub untagged: Option<bool>,
    /// How `amount` (and, for `Between`, `amount_max`) narrow the list. Ignored without `amount`.
    pub amount_op: Option<AmountOp>,
    pub amount: Option<Decimal>,
    /// Inclusive upper bound, only read when `amount_op` is `Between`.
    pub amount_max: Option<Decimal>,
}

impl TransactionFilter {
    /// Resolves `amount_op`/`amount`/`amount_max` into `(greater than, less than, at least, at
    /// most)` bounds on the (always positive) transaction amount — `None` meaning "no bound".
    pub fn amount_bounds(&self) -> [Option<Decimal>; 4] {
        match (self.amount_op, self.amount) {
            (Some(AmountOp::GreaterThan), Some(amount)) => [Some(amount), None, None, None],
            (Some(AmountOp::LessThan), Some(amount)) => [None, Some(amount), None, None],
            (Some(AmountOp::EqualTo), Some(amount)) => [None, None, Some(amount), Some(amount)],
            (Some(AmountOp::Between), Some(amount)) => [None, None, Some(amount), self.amount_max],
            _ => [None, None, None, None],
        }
    }
}

/// Comparison applied by `TransactionFilter::amount`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmountOp {
    GreaterThan,
    LessThan,
    EqualTo,
    /// Inclusive on both ends.
    Between,
}

/// Parses a comma-separated query param like `3,7,12` into ids. Query strings can't carry a
/// sequence natively through `web::Query`, hence the single delimited value. An empty value means
/// "no filter", same as leaving the param out.
fn deserialize_id_list<'de, D>(deserializer: D) -> Result<Option<Vec<i32>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let Some(raw) = Option::<String>::deserialize(deserializer)? else {
        return Ok(None);
    };
    if raw.trim().is_empty() {
        return Ok(None);
    }
    raw.split(',')
        .map(|id| id.trim().parse::<i32>().map_err(serde::de::Error::custom))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
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
    pub account_id: Option<i32>,
    /// `None` leaves tags unchanged. `Some(ids)` *replaces* the full tag set with exactly `ids`
    /// (so `Some([])` clears every tag) — unlike `BulkTransactionPatch::tag_ids`, which only adds.
    /// Ids outside the caller's own household are silently dropped.
    pub tag_ids: Option<Vec<i32>>,
}

/// The subset of [`TransactionPatch`] fields the bulk "edit multiple" panel exposes — no `amount`,
/// since setting one dollar amount across several different transactions isn't a sensible bulk
/// operation.
#[derive(Debug, Deserialize)]
pub struct BulkTransactionPatch {
    pub date: Option<NaiveDate>,
    pub merchant_id: Option<i32>,
    pub category_id: Option<i32>,
    pub account_id: Option<i32>,
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
