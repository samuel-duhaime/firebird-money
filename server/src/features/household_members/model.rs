use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `household_members` table: connects a `User` to a `Household` with a role.
#[derive(Debug, Serialize, FromRow)]
pub struct HouseholdMember {
    pub id: i32,
    pub household_id: i32,
    pub user_id: i32,
    pub r#type: String,
    pub created_at: DateTime<Utc>,
}

/// Body for `POST /household-members`. `id` and `created_at` are generated.
#[derive(Debug, Deserialize)]
pub struct NewHouseholdMember {
    pub household_id: i32,
    pub user_id: i32,
    pub r#type: String,
}

/// Body for `PATCH /household-members/{id}`. Only the role can change — `household_id` and
/// `user_id` are fixed at creation; delete and recreate the membership to move it.
#[derive(Debug, Deserialize)]
pub struct HouseholdMemberPatch {
    pub r#type: Option<String>,
}

/// Optional query params for `GET /household-members`. Absent fields mean "no filter".
#[derive(Debug, Deserialize)]
pub struct HouseholdMemberFilter {
    pub household_id: Option<i32>,
    pub user_id: Option<i32>,
}
