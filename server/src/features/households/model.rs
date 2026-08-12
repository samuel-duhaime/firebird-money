use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

/// A single row in the `households` table. Carries no identity of its own — see
/// `household_members` for who belongs to it and how.
#[derive(Debug, Serialize, FromRow)]
pub struct Household {
    pub id: i32,

    /// Short code an existing member shares so someone else can join this household during
    /// onboarding. Generated on creation and never changes.
    pub join_code: String,

    pub created_at: DateTime<Utc>,
}
