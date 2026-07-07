use sqlx::PgPool;

use super::model::{Category, NewCategory};

const SELECT_COLUMNS: &str = "id, name_en, name_fr, type, created_at";

/// Inserts a new category and returns the created row.
pub async fn create(pool: &PgPool, new_category: &NewCategory) -> Result<Category, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "INSERT INTO categories (name_en, name_fr, type)
         VALUES ($1, $2, $3)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(&new_category.name_en)
    .bind(&new_category.name_fr)
    .bind(&new_category.r#type)
    .fetch_one(pool)
    .await
}
