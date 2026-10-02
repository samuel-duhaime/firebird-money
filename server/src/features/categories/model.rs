use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `categories` table. `type` (income/expense/transfer) lives on its
/// `category_groups` row, not here — every category in a group shares its type.
#[derive(Debug, Serialize, FromRow)]
pub struct Category {
    pub id: i32,
    pub household_id: i32,
    pub group_id: i32,
    pub name_en: String,
    pub name_fr: String,
    pub created_at: DateTime<Utc>,
    /// This household's chosen display order for this category among the others in its group (see
    /// `PATCH /categories/reorder`) — lower sorts first. Only meaningful relative to the other
    /// categories sharing its `group_id`, not a household-wide rank.
    pub sort_order: i32,
}

/// Body for `POST /categories`. `id` and `created_at` are generated. `household_id` is never read
/// from the body — it's always the caller's own household, from `CurrentUser`.
#[derive(Debug, Deserialize)]
pub struct NewCategory {
    pub group_id: i32,
    pub name_en: String,
    pub name_fr: String,
}

/// Body for `PATCH /categories/{id}`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct CategoryPatch {
    pub group_id: Option<i32>,
    pub name_en: Option<String>,
    pub name_fr: Option<String>,
}

/// Body for `PATCH /categories/reorder` — a set of categories, in their new display order. Every
/// id must belong to the caller's own household (ids that don't are silently skipped, like
/// elsewhere in this API). Only the given ids' `sort_order` changes: pass just one group's category
/// ids to reorder within that group alone, leaving every other group untouched.
#[derive(Debug, Deserialize)]
pub struct ReorderCategoriesRequest {
    pub category_ids: Vec<i32>,
}
