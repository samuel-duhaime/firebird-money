use sqlx::PgConnection;
use sqlx::PgPool;

use super::defaults::DefaultCategory;
use super::model::{Category, CategoryPatch, NewCategory};

const SELECT_COLUMNS: &str = "id, group_id, name_en, name_fr, created_at";

/// Inserts the starter categories for one newly created default group, as part of
/// `category_groups::repository::seed_defaults`. Takes the transaction connection directly (rather
/// than a generic `PgExecutor`) since the caller reborrows it once per group in a loop.
pub async fn insert_defaults(
    tx: &mut PgConnection,
    household_id: i32,
    group_id: i32,
    categories: &[DefaultCategory],
) -> Result<(), sqlx::Error> {
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO categories (household_id, group_id, name_en, name_fr) ",
    );
    query_builder.push_values(categories, |mut row, category| {
        row.push_bind(household_id)
            .push_bind(group_id)
            .push_bind(category.name_en)
            .push_bind(category.name_fr);
    });
    query_builder.build().execute(&mut *tx).await?;
    Ok(())
}

/// Inserts a new category and returns the created row.
pub async fn create(pool: &PgPool, new_category: &NewCategory) -> Result<Category, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "INSERT INTO categories (group_id, name_en, name_fr)
         VALUES ($1, $2, $3)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(new_category.group_id)
    .bind(&new_category.name_en)
    .bind(&new_category.name_fr)
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
             group_id = COALESCE($4, group_id)
         WHERE id = $1
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(&patch.name_en)
    .bind(&patch.name_fr)
    .bind(patch.group_id)
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
