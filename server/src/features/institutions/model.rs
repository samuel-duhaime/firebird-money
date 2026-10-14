use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `institutions` table — the bank, credit union, or brokerage an account is
/// held at. An institution with `household_id: None` is *common*: shared by every household, seeded
/// at startup from `defaults::DEFAULT_INSTITUTIONS`, and immune to rename/delete through this
/// household-scoped API (see `repository::update`/`delete`). Everything else belongs to exactly one
/// household.
#[derive(Debug, Serialize, FromRow)]
pub struct Institution {
    pub id: i32,
    pub household_id: Option<i32>,
    pub name: String,
    /// Other names this institution is searchable by (e.g. "Banque Nationale" for "National
    /// Bank"). Empty for custom ones — not settable through this API.
    pub aliases: Vec<String>,
    pub website_url: Option<String>,
    pub logo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    /// How many of the caller's household's accounts are held here — always computed fresh.
    pub account_count: i64,
}

/// Body for `POST /institutions`. `household_id` is never read from the body — it's always the
/// caller's own household, from `CurrentUser`.
#[derive(Debug, Deserialize)]
pub struct NewInstitution {
    pub name: String,
    pub website_url: Option<String>,
}

/// Body for `PATCH /institutions/{id}`. `None` fields are left unchanged; `website_url: ""` clears
/// it (see `handlers::update_institution`). Only ever takes effect on an institution the caller's
/// household owns.
#[derive(Debug, Deserialize)]
pub struct InstitutionPatch {
    pub name: Option<String>,
    pub website_url: Option<String>,
}
