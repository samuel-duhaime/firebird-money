use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `categories` table.
#[derive(Debug, Serialize, FromRow)]
pub struct Category {
    pub id: i32,
    pub name_en: String,
    pub name_fr: String,
    pub r#type: String,
    pub created_at: DateTime<Utc>,
}

/// Body for `POST /categories`. All fields are required; `id` and `created_at` are generated.
#[derive(Debug, Deserialize)]
pub struct NewCategory {
    pub name_en: String,
    pub name_fr: String,
    pub r#type: String,
}

/// Body for `PATCH /categories/{id}`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct CategoryPatch {
    pub name_en: Option<String>,
    pub name_fr: Option<String>,
    pub r#type: Option<String>,
}
