use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

/// A single row in the `households` table. Carries no identity of its own — see
/// `household_members` for who belongs to it and how.
///
/// `join_code` is a shared secret (it's what lets someone join the household), and there's no
/// auth guard yet on `GET /households/{id}` to check the caller is a member (that lands in #65).
/// Serializing `Household` directly is fine for `POST /households` — the caller just created it —
/// but any route without an auth check should serialize `PublicHousehold` instead.
#[derive(Debug, Serialize, FromRow)]
pub struct Household {
    pub id: i32,
    pub join_code: String,
    pub created_at: DateTime<Utc>,
}

/// `Household` without `join_code`, for responses that aren't scoped to an authenticated member.
#[derive(Debug, Serialize)]
pub struct PublicHousehold {
    pub id: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Household> for PublicHousehold {
    fn from(household: Household) -> Self {
        Self {
            id: household.id,
            created_at: household.created_at,
        }
    }
}
