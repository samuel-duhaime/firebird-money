use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `categories` table. `type` (income/expense/transfer) lives on its
/// `category_groups` row, not here — every category in a group shares its type.
#[derive(Debug, Serialize, FromRow)]
pub struct Category {
    pub id: i32,
    pub group_id: i32,
    pub name_en: String,
    pub name_fr: String,
    pub created_at: DateTime<Utc>,
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
