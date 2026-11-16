use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;

/// A signed-in user's display preferences: their UI language, which optional columns show on the
/// transactions list, and the dashboard's widgets. One row per user; see `repository::update` for
/// how it's created.
#[derive(Debug, Serialize, FromRow)]
pub struct Settings {
    pub user_id: i32,
    pub show_category_column: bool,
    pub show_tags_column: bool,
    pub show_account_column: bool,
    pub show_owner_column: bool,
    /// `None` until the user picks a language.
    pub language: Option<Language>,
    /// Every dashboard widget, in display order — see `normalize_dashboard_widgets`.
    pub dashboard_widgets: Json<Vec<DashboardWidget>>,
}

/// A widget the dashboard can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardWidgetId {
    Transactions,
}

impl DashboardWidgetId {
    /// Every widget, in the order a new user sees them.
    pub const ALL: [DashboardWidgetId; 1] = [DashboardWidgetId::Transactions];
}

/// One dashboard widget, shown or hidden; its place in the list is its place on the dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub id: DashboardWidgetId,
    pub visible: bool,
}

/// The saved widgets, in their saved order without repeats, then every widget the list doesn't
/// mention, shown — so a widget added later shows for everyone until they hide it.
pub fn normalize_dashboard_widgets(saved: &[DashboardWidget]) -> Vec<DashboardWidget> {
    let mut widgets: Vec<DashboardWidget> = Vec::with_capacity(DashboardWidgetId::ALL.len());
    for widget in saved {
        if !widgets.iter().any(|w| w.id == widget.id) {
            widgets.push(*widget);
        }
    }
    for id in DashboardWidgetId::ALL {
        if !widgets.iter().any(|w| w.id == id) {
            widgets.push(DashboardWidget { id, visible: true });
        }
    }
    widgets
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
    /// What a user who has never saved any settings sees: every optional column and dashboard
    /// widget visible, no language picked yet.
    pub fn defaults(user_id: i32) -> Self {
        Self {
            user_id,
            show_category_column: true,
            show_tags_column: true,
            show_account_column: true,
            show_owner_column: true,
            language: None,
            dashboard_widgets: Json(normalize_dashboard_widgets(&[])),
        }
    }
}

/// Body for `PATCH /settings`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct SettingsPatch {
    pub show_category_column: Option<bool>,
    pub show_tags_column: Option<bool>,
    pub show_account_column: Option<bool>,
    pub show_owner_column: Option<bool>,
    pub language: Option<Language>,
    /// The whole list, in its new order; a widget it leaves out shows after the others.
    pub dashboard_widgets: Option<Vec<DashboardWidget>>,
}
