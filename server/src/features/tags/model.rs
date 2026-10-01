use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `tags` table: a free-form label the household can attach to transactions,
/// independent of category or account.
#[derive(Debug, Serialize, FromRow)]
pub struct Tag {
    pub id: i32,
    pub household_id: i32,
    pub name: String,
    pub color: String,
    pub created_at: DateTime<Utc>,
    /// How many transactions currently carry this tag — always computed fresh from
    /// `transaction_tags`, never stored.
    pub transaction_count: i64,
    /// This household's chosen display order for its tags (see `PATCH /tags/reorder`) — lower
    /// sorts first. Not a global rank; only meaningful relative to the household's other tags.
    pub sort_order: i32,
}

/// Body for `POST /tags`. `id`, `created_at`, and `sort_order` are generated — a new tag always
/// starts at the end of the household's order. `household_id` is never read from the body — it's
/// always the caller's own household, from `CurrentUser`.
#[derive(Debug, Deserialize)]
pub struct NewTag {
    pub name: String,
    pub color: String,
}

/// Body for `PATCH /tags/{id}`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct TagPatch {
    pub name: Option<String>,
    pub color: Option<String>,
}

/// Body for `PATCH /tags/reorder` — the household's tags, in the new display order. Every id must
/// belong to the caller's own household (ids that don't are silently skipped, like elsewhere in
/// this API), and this is a full replacement of the order, not a patch: a tag left out of `tag_ids`
/// keeps whatever `sort_order` it already had.
#[derive(Debug, Deserialize)]
pub struct ReorderTagsRequest {
    pub tag_ids: Vec<i32>,
}
