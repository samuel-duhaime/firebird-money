use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `merchants` table — who a transaction was paid to/by, e.g. "Starbucks",
/// "IGA". A merchant with `household_id: None` is *common*: shared by every household, seeded once
/// by the `create_merchants` migration, and immune to rename/delete through this household-scoped
/// API (see `repository::update`/`delete`). Everything else belongs to exactly one household, same
/// as categories/tags.
#[derive(Debug, Serialize, FromRow)]
pub struct Merchant {
    pub id: i32,
    pub household_id: Option<i32>,
    pub name: String,
    /// Alternate spellings this merchant also matches on (typed search, and raw statement text
    /// during auto-matching) without changing `name` itself — see `defaults::DefaultMerchant`. Empty
    /// for most merchants, including every custom one (not settable through this API in this PR).
    pub aliases: Vec<String>,
    pub logo_url: Option<String>,
    /// The category most often used with this merchant in the caller's household's own
    /// transactions — not stored anywhere, always recomputed from `transactions` (see
    /// `repository::RECOMMENDED_CATEGORY_SUBQUERY`): the category with the most transactions for
    /// this merchant, ties broken by whichever was used most recently. `None` until this household
    /// has at least one transaction with this merchant.
    pub recommended_category_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    /// How many of the caller's household's transactions currently carry this merchant — always
    /// computed fresh from `transactions`, never stored.
    pub transaction_count: i64,
}

/// Body for `POST /merchants`. `id` and `created_at` are generated; `household_id` is never read
/// from the body — it's always the caller's own household, from `CurrentUser`. There's no
/// `recommended_category_id` here — it's never set directly, only derived from transaction history.
#[derive(Debug, Deserialize)]
pub struct NewMerchant {
    pub name: String,
}

/// Body for `PATCH /merchants/{id}`. `None` fields are left unchanged. Only ever takes effect on a
/// merchant the caller's household owns — see `repository::update`.
#[derive(Debug, Deserialize)]
pub struct MerchantPatch {
    pub name: Option<String>,
}

/// Sort order for `GET /merchants`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MerchantSortOrder {
    /// Most transactions first (default) — matches how the settings page is meant to be scanned:
    /// merchants you actually use float to the top of what could otherwise be a long common list.
    TransactionCount,
    /// A-Z by name.
    Alphabetical,
}

/// Optional query params for `GET /merchants`. Absent `order` defaults to `TransactionCount`.
#[derive(Debug, Deserialize)]
pub struct MerchantFilter {
    pub order: Option<MerchantSortOrder>,
}
