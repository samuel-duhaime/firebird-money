use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A user seen as a member of their household — what `/household-members` returns, so the members
/// page can show each person's role, name, email, and account status. `id` is the user's id.
#[derive(Debug, Serialize, FromRow)]
pub struct HouseholdMember {
    pub id: i32,
    pub household_id: i32,
    pub role: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Body for `PATCH /household-members/{id}`. Only the role can change.
#[derive(Debug, Deserialize)]
pub struct HouseholdMemberPatch {
    pub role: Option<String>,
}
