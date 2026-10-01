use sqlx::PgConnection;
use sqlx::PgPool;

use super::defaults::DEFAULT_TAGS;
use super::model::{NewTag, Tag, TagPatch};

const SELECT_COLUMNS: &str = "id, household_id, name, color, created_at, sort_order";

/// Seeds the standard starter tags for a newly created household, in `DEFAULT_TAGS` order. Takes
/// the transaction connection directly so `households::repository::create` can run this alongside
/// the household's own insert and commit both together, never leaving a household without its
/// starter tags.
pub async fn seed_defaults(tx: &mut PgConnection, household_id: i32) -> Result<(), sqlx::Error> {
    let mut query_builder =
        sqlx::QueryBuilder::new("INSERT INTO tags (household_id, name, color, sort_order) ");
    query_builder.push_values(DEFAULT_TAGS.iter().enumerate(), |mut row, (index, tag)| {
        row.push_bind(household_id)
            .push_bind(tag.name)
            .push_bind(tag.color)
            .push_bind(index as i32);
    });
    query_builder.build().execute(&mut *tx).await?;
    Ok(())
}

/// Inserts a new tag, scoped to `household_id`, at the end of the household's order, and returns
/// the created row. `transaction_count` is always 0 for a just-created tag, but computed the same
/// way as everywhere else for consistency rather than hardcoded.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    new_tag: &NewTag,
) -> Result<Tag, sqlx::Error> {
    sqlx::query_as::<_, Tag>(&format!(
        "WITH inserted AS (
            INSERT INTO tags (household_id, name, color, sort_order)
            VALUES (
                $1, $2, $3,
                (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM tags WHERE household_id = $1)
            )
            RETURNING *
         )
         SELECT {SELECT_COLUMNS},
                (SELECT COUNT(*) FROM transaction_tags WHERE tag_id = inserted.id) AS transaction_count
         FROM inserted"
    ))
    .bind(household_id)
    .bind(&new_tag.name)
    .bind(&new_tag.color)
    .fetch_one(pool)
    .await
}

/// Lists a household's tags in their display order (see `reorder`), each with how many
/// transactions currently carry it.
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(&format!(
        "SELECT {SELECT_COLUMNS},
                (SELECT COUNT(*) FROM transaction_tags WHERE tag_id = tags.id) AS transaction_count
         FROM tags
         WHERE household_id = $1
         ORDER BY sort_order, id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single tag by id, scoped to `household_id`, or `None` if it doesn't exist (including
/// when it belongs to a different household).
pub async fn get(pool: &PgPool, household_id: i32, id: i32) -> Result<Option<Tag>, sqlx::Error> {
    sqlx::query_as::<_, Tag>(&format!(
        "SELECT {SELECT_COLUMNS},
                (SELECT COUNT(*) FROM transaction_tags WHERE tag_id = tags.id) AS transaction_count
         FROM tags
         WHERE id = $1 AND household_id = $2"
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
        "WITH updated AS (
            UPDATE tags
            SET name = COALESCE($3, name),
                color = COALESCE($4, color)
            WHERE id = $1 AND household_id = $2
            RETURNING *
         )
         SELECT {SELECT_COLUMNS},
                (SELECT COUNT(*) FROM transaction_tags WHERE tag_id = updated.id) AS transaction_count
         FROM updated"
    ))
    .bind(id)
    .bind(household_id)
    .bind(&patch.name)
    .bind(&patch.color)
    .fetch_optional(pool)
    .await
}

/// Sets each id in `tag_ids`' `sort_order` to its index in that list, scoped to `household_id` —
/// ids that don't exist or belong to a different household are silently skipped. A full
/// replacement of the order: a tag left out of `tag_ids` keeps its existing `sort_order`.
pub async fn reorder(pool: &PgPool, household_id: i32, tag_ids: &[i32]) -> Result<(), sqlx::Error> {
    if tag_ids.is_empty() {
        return Ok(());
    }

    let positions: Vec<i32> = (0..tag_ids.len() as i32).collect();
    sqlx::query(
        "UPDATE tags
         SET sort_order = data.position
         FROM UNNEST($1::integer[], $2::integer[]) AS data(id, position)
         WHERE tags.id = data.id AND tags.household_id = $3",
    )
    .bind(tag_ids)
    .bind(&positions)
    .bind(household_id)
    .execute(pool)
    .await?;

    Ok(())
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
