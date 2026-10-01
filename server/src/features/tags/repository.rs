use sqlx::PgConnection;
use sqlx::PgPool;

use super::defaults::DEFAULT_TAGS;
use super::model::{NewTag, Tag, TagPatch};

const SELECT_COLUMNS: &str = "id, household_id, name, color, created_at";

/// Seeds the standard starter tags for a newly created household. Takes the transaction
/// connection directly so `households::repository::create` can run this alongside the household's
/// own insert and commit both together, never leaving a household without its starter tags.
pub async fn seed_defaults(tx: &mut PgConnection, household_id: i32) -> Result<(), sqlx::Error> {
    let mut query_builder =
        sqlx::QueryBuilder::new("INSERT INTO tags (household_id, name, color) ");
    query_builder.push_values(DEFAULT_TAGS, |mut row, tag| {
        row.push_bind(household_id)
            .push_bind(tag.name)
            .push_bind(tag.color);
    });
    query_builder.build().execute(&mut *tx).await?;
    Ok(())
}

/// Inserts a new tag, scoped to `household_id`, and returns the created row.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    new_tag: &NewTag,
) -> Result<Tag, sqlx::Error> {
    sqlx::query_as::<_, Tag>(&format!(
        "INSERT INTO tags (household_id, name, color)
         VALUES ($1, $2, $3)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(household_id)
    .bind(&new_tag.name)
    .bind(&new_tag.color)
    .fetch_one(pool)
    .await
}

/// Lists a household's tags, ordered by id.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(&format!(
        "SELECT {SELECT_COLUMNS} FROM tags WHERE household_id = $1 ORDER BY id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single tag by id, scoped to `household_id`, or `None` if it doesn't exist (including
/// when it belongs to a different household).
pub async fn get(pool: &PgPool, household_id: i32, id: i32) -> Result<Option<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(&format!(
        "SELECT {SELECT_COLUMNS} FROM tags WHERE id = $1 AND household_id = $2"
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
    patch: &TagPatch,
) -> Result<Option<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(&format!(
        "UPDATE tags
         SET name = COALESCE($3, name),
             color = COALESCE($4, color)
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(&patch.name)
    .bind(&patch.color)
    .fetch_optional(pool)
    .await
}

/// Deletes a tag by id, scoped to `household_id`. Returns `true` if a row was deleted, `false` if
/// the id didn't exist (including when it belongs to a different household).
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM tags WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
