use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Every value `accounts.type` accepts — mirrors the `accounts_type_valid` CHECK constraint.
pub const ACCOUNT_TYPES: &[&str] = &[
    "checking",
    "savings",
    "credit_card",
    "investment",
    "loan",
    "mortgage",
    "cash",
    "other",
];

/// A single row in the `accounts` table, joined with its institution's name — a household's bank
/// account, credit card, investment account, ... Always belongs to exactly one household and is held
/// at exactly one institution (common, or the household's own custom one).
#[derive(Debug, Serialize, FromRow)]
pub struct Account {
    pub id: i32,
    pub household_id: i32,
    pub institution_id: i32,
    pub institution_name: String,
    pub name: String,
    /// One of `ACCOUNT_TYPES`.
    pub r#type: String,
    /// The last few characters of the account number, shown as "(…5012)".
    pub last_four: Option<String>,
    /// ISO 4217 code, e.g. "CAD".
    pub currency: String,
    pub created_at: DateTime<Utc>,
    /// How many of the household's transactions are in this account — always computed fresh.
    pub transaction_count: i64,
}

/// Body for `POST /accounts`. `household_id` is never read from the body — it's always the caller's
/// own household, from `CurrentUser`. `currency` defaults to `CAD` when absent.
#[derive(Debug, Deserialize)]
pub struct NewAccount {
    pub institution_id: i32,
    pub name: String,
    pub r#type: String,
    pub last_four: Option<String>,
    pub currency: Option<String>,
}

/// Body for `PATCH /accounts/{id}`. `None` fields are left unchanged; `last_four: ""` clears it.
#[derive(Debug, Deserialize)]
pub struct AccountPatch {
    pub institution_id: Option<i32>,
    pub name: Option<String>,
    pub r#type: Option<String>,
    pub last_four: Option<String>,
    pub currency: Option<String>,
}
