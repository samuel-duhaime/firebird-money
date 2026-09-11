use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `category_groups` table: an umbrella a household's categories are
/// organized under (e.g. "Food & Dining"), carrying the `type` every category inside it shares.
#[derive(Debug, Serialize, FromRow)]
pub struct CategoryGroup {
    pub id: i32,
    pub household_id: i32,
    pub name_en: String,
    pub name_fr: String,
    pub r#type: String,
    pub created_at: DateTime<Utc>,
}

/// Body for `POST /category-groups`. `household_id` is never read from the body — it's always the
/// caller's own household, from `CurrentUser`.
#[derive(Debug, Deserialize)]
pub struct NewCategoryGroup {
    pub name_en: String,
    pub name_fr: String,
    pub r#type: String,
}

/// Body for `PATCH /category-groups/{id}`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct CategoryGroupPatch {
    pub name_en: Option<String>,
    pub name_fr: Option<String>,
    pub r#type: Option<String>,
}
