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
}

/// Body for `POST /tags`. `id` and `created_at` are generated. `household_id` is never read from
/// the body — it's always the caller's own household, from `CurrentUser`.
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
