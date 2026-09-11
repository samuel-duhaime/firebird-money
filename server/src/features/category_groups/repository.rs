use sqlx::PgPool;

use super::defaults::DEFAULT_CATEGORY_GROUPS;
use super::model::{CategoryGroup, CategoryGroupPatch, NewCategoryGroup};
use crate::features::categories::repository as categories_repository;

const SELECT_COLUMNS: &str = "id, household_id, name_en, name_fr, type, created_at";

/// Seeds the standard starter groups (and the categories inside each) for a newly created
/// household. Takes the transaction connection directly so `households::repository::create` can
/// run this alongside the household's own insert and commit both together, never leaving a
/// household without its starter categories.
pub async fn seed_defaults(
    tx: &mut sqlx::PgConnection,
    household_id: i32,
) -> Result<(), sqlx::Error> {
    for group in DEFAULT_CATEGORY_GROUPS {
        let (group_id,): (i32,) = sqlx::query_as(
            "INSERT INTO category_groups (household_id, name_en, name_fr, type)
             VALUES ($1, $2, $3, $4)
             RETURNING id",
        )
        .bind(household_id)
        .bind(group.name_en)
        .bind(group.name_fr)
        .bind(group.r#type)
        .fetch_one(&mut *tx)
        .await?;

        categories_repository::insert_defaults(tx, household_id, group_id, group.categories)
            .await?;
    }
    Ok(())
}

/// Inserts a new category group, scoped to `household_id`, and returns the created row.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    new_group: &NewCategoryGroup,
) -> Result<CategoryGroup, sqlx::Error> {
    sqlx::query_as::<_, CategoryGroup>(&format!(
        "INSERT INTO category_groups (household_id, name_en, name_fr, type)
         VALUES ($1, $2, $3, $4)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(household_id)
    .bind(&new_group.name_en)
    .bind(&new_group.name_fr)
    .bind(&new_group.r#type)
    .fetch_one(pool)
    .await
}

/// Lists a household's category groups, ordered by id.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<CategoryGroup>, sqlx::Error> {
    sqlx::query_as::<_, CategoryGroup>(&format!(
        "SELECT {SELECT_COLUMNS} FROM category_groups WHERE household_id = $1 ORDER BY id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single category group by id, scoped to `household_id`, or `None` if it doesn't exist
/// (including when it belongs to a different household).
pub async fn get(
    pool: &PgPool,
    household_id: i32,
    id: i32,
) -> Result<Option<CategoryGroup>, sqlx::Error> {
    sqlx::query_as::<_, CategoryGroup>(&format!(
        "SELECT {SELECT_COLUMNS} FROM category_groups WHERE id = $1 AND household_id = $2"
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
    patch: &CategoryGroupPatch,
) -> Result<Option<CategoryGroup>, sqlx::Error> {
    sqlx::query_as::<_, CategoryGroup>(&format!(
        "UPDATE category_groups
         SET name_en = COALESCE($3, name_en),
             name_fr = COALESCE($4, name_fr),
             type = COALESCE($5, type)
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(&patch.name_en)
    .bind(&patch.name_fr)
    .bind(&patch.r#type)
    .fetch_optional(pool)
    .await
}

/// Deletes a category group by id, scoped to `household_id`. Returns `true` if a row was deleted,
/// `false` if the id didn't exist (including when it belongs to a different household).
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM category_groups WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
