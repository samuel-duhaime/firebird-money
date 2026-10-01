use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A signed-in user's display preferences — currently just which optional columns show on the
/// transactions list. One row per user; see `repository::update` for how it's created.
#[derive(Debug, Serialize, FromRow)]
pub struct Settings {
    pub user_id: i32,
    pub show_category_column: bool,
    pub show_tags_column: bool,
    pub show_account_column: bool,
}

impl Settings {
    /// What a user who has never saved any settings sees: every optional column visible.
    pub fn defaults(user_id: i32) -> Self {
        Self {
            user_id,
            show_category_column: true,
            show_tags_column: true,
            show_account_column: true,
        }
    }
}

/// Body for `PATCH /settings`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct SettingsPatch {
    pub show_category_column: Option<bool>,
    pub show_tags_column: Option<bool>,
    pub show_account_column: Option<bool>,
}
