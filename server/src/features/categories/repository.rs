use sqlx::PgPool;

use super::defaults::DEFAULT_CATEGORIES;
use super::model::{Category, CategoryPatch, NewCategory};

const SELECT_COLUMNS: &str = "id, name_en, name_fr, type, created_at";

/// Seeds the standard starter categories for a newly created household. Takes any `PgExecutor` —
/// in practice a transaction handle — so `households::repository::create` can run this alongside
/// the household's own insert and commit both together, never leaving a household without them.
pub async fn seed_defaults<'e, E>(executor: E, household_id: i32) -> Result<(), sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    let mut query_builder =
        sqlx::QueryBuilder::new("INSERT INTO categories (household_id, name_en, name_fr, type) ");
    query_builder.push_values(DEFAULT_CATEGORIES, |mut row, category| {
        row.push_bind(household_id)
            .push_bind(category.name_en)
            .push_bind(category.name_fr)
            .push_bind(category.r#type);
    });
    query_builder.build().execute(executor).await?;
    Ok(())
}

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

/// Lists all categories, ordered by id.
pub async fn list(pool: &PgPool) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "SELECT {SELECT_COLUMNS} FROM categories ORDER BY id"
    ))
    .fetch_all(pool)
    .await
}

/// Fetches a single category by id, or `None` if it doesn't exist.
pub async fn get(pool: &PgPool, id: i32) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "SELECT {SELECT_COLUMNS} FROM categories WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update (only `Some` fields change) and returns the updated row, or `None` if
/// the id doesn't exist.
pub async fn update(
    pool: &PgPool,
    id: i32,
    patch: &CategoryPatch,
) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "UPDATE categories
         SET name_en = COALESCE($2, name_en),
             name_fr = COALESCE($3, name_fr),
             type = COALESCE($4, type)
         WHERE id = $1
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(&patch.name_en)
    .bind(&patch.name_fr)
    .bind(&patch.r#type)
    .fetch_optional(pool)
    .await
}

/// Deletes a category by id. Returns `true` if a row was deleted, `false` if the id didn't exist.
pub async fn delete(pool: &PgPool, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM categories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
