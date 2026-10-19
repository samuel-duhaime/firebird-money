use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A signed-in user's display preferences: their UI language and which optional columns show on
/// the transactions list. One row per user; see `repository::update` for how it's created.
#[derive(Debug, Serialize, FromRow)]
pub struct Settings {
    pub user_id: i32,
    pub show_category_column: bool,
    pub show_tags_column: bool,
    pub show_account_column: bool,
    /// `None` until the user picks a language.
    pub language: Option<Language>,
}

/// A UI language the client ships translations for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Fr,
}

impl Settings {
    /// What a user who has never saved any settings sees: every optional column visible, no
    /// language picked yet.
    pub fn defaults(user_id: i32) -> Self {
        Self {
            user_id,
            show_category_column: true,
            show_tags_column: true,
            show_account_column: true,
            language: None,
        }
    }
}

/// Body for `PATCH /settings`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct SettingsPatch {
    pub show_category_column: Option<bool>,
    pub show_tags_column: Option<bool>,
    pub show_account_column: Option<bool>,
    pub language: Option<Language>,
}
