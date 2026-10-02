use sqlx::PgConnection;
use sqlx::PgPool;

use super::defaults::DefaultCategory;
use super::model::{Category, CategoryPatch, NewCategory};

const SELECT_COLUMNS: &str = "id, household_id, group_id, name_en, name_fr, created_at, sort_order";

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
        "INSERT INTO categories (household_id, group_id, name_en, name_fr, sort_order) ",
    );
    query_builder.push_values(
        categories.iter().enumerate(),
        |mut row, (index, category)| {
            row.push_bind(household_id)
                .push_bind(group_id)
                .push_bind(category.name_en)
                .push_bind(category.name_fr)
                .push_bind(index as i32);
        },
    );
    query_builder.build().execute(&mut *tx).await?;
    Ok(())
}

/// Inserts a new category, scoped to `household_id`, at the end of its group's order, and returns
/// the created row.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    new_category: &NewCategory,
) -> Result<Category, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "INSERT INTO categories (household_id, group_id, name_en, name_fr, sort_order)
         VALUES (
             $1, $2, $3, $4,
             (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories WHERE group_id = $2)
         )
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(household_id)
    .bind(new_category.group_id)
    .bind(&new_category.name_en)
    .bind(&new_category.name_fr)
    .fetch_one(pool)
    .await
}

/// Lists a household's categories, each in its display order relative to the others in its group
/// (see `reorder`).
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "SELECT {SELECT_COLUMNS} FROM categories WHERE household_id = $1 ORDER BY group_id, sort_order, id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Sets each id in `category_ids`' `sort_order` to its index in that list, scoped to
/// `household_id` — ids that don't exist or belong to a different household are silently skipped.
/// Pass only one group's category ids to reorder within that group alone: a category left out of
/// `category_ids` (including every category in another group) keeps its existing `sort_order`.
pub async fn reorder(
    pool: &PgPool,
    household_id: i32,
    category_ids: &[i32],
) -> Result<(), sqlx::Error> {
    if category_ids.is_empty() {
        return Ok(());
    }

    let positions: Vec<i32> = (0..category_ids.len() as i32).collect();
    sqlx::query(
        "UPDATE categories
         SET sort_order = data.position
         FROM UNNEST($1::integer[], $2::integer[]) AS data(id, position)
         WHERE categories.id = data.id AND categories.household_id = $3",
    )
    .bind(category_ids)
    .bind(&positions)
    .bind(household_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetches a single category by id, scoped to `household_id`, or `None` if it doesn't exist
/// (including when it belongs to a different household).
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "SELECT {SELECT_COLUMNS} FROM categories WHERE id = $1 AND household_id = $2"
    ))
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update (only `Some` fields change), scoped to `household_id`, and returns the
/// updated row, or `None` if the id doesn't exist (including when it belongs to a different
/// household).
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    patch: &CategoryPatch,
) -> Result<Option<Category>, sqlx::Error> {
    sqlx::query_as::<_, Category>(&format!(
        "UPDATE categories
         SET name_en = COALESCE($3, name_en),
             name_fr = COALESCE($4, name_fr),
             group_id = COALESCE($5, group_id)
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(&patch.name_en)
    .bind(&patch.name_fr)
    .bind(patch.group_id)
    .fetch_optional(pool)
    .await
}

/// Deletes a category by id, scoped to `household_id`. Returns `true` if a row was deleted,
/// `false` if the id didn't exist (including when it belongs to a different household).
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM categories WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
