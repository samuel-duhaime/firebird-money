use sqlx::PgPool;

use super::model::{Settings, SettingsPatch};

const SELECT_COLUMNS: &str = "user_id, show_category_column, show_tags_column, show_account_column";

/// Fetches the user's settings row, or `None` if they've never saved any — callers fall back to
/// `Settings::defaults` in that case.
pub async fn get(pool: &PgPool, user_id: i32) -> Result<Option<Settings>, sqlx::Error> {
    sqlx::query_as::<_, Settings>(&format!(
        "SELECT {SELECT_COLUMNS} FROM settings WHERE user_id = $1"
    ))
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

/// Applies a partial update, creating the row (at defaults) on the user's first write. Fields left
/// `None` in `patch` keep their current (or default) value.
pub async fn update(
    pool: &PgPool,
    user_id: i32,
    patch: &SettingsPatch,
) -> Result<Settings, sqlx::Error> {
    sqlx::query_as::<_, Settings>(&format!(
        "INSERT INTO settings (user_id, show_category_column, show_tags_column, show_account_column)
         VALUES ($1, COALESCE($2, true), COALESCE($3, true), COALESCE($4, true))
         ON CONFLICT (user_id) DO UPDATE
         SET show_category_column = COALESCE($2, settings.show_category_column),
             show_tags_column = COALESCE($3, settings.show_tags_column),
             show_account_column = COALESCE($4, settings.show_account_column)
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(user_id)
    .bind(patch.show_category_column)
    .bind(patch.show_tags_column)
    .bind(patch.show_account_column)
    .fetch_one(pool)
    .await
}
