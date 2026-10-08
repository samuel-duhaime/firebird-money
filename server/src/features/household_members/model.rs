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

/// A `HouseholdMember` plus who the member is — what `GET /household-members` returns, so the
/// members page can show each person's name, email, and account status without a call per user.
#[derive(Debug, Serialize, FromRow)]
pub struct HouseholdMemberWithUser {
    pub id: i32,
    pub household_id: i32,
    pub user_id: i32,
    pub r#type: String,
    pub created_at: DateTime<Utc>,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub status: String,
}

/// Body for `POST /household-members`. `id` and `created_at` are generated; `household_id` is
/// never read from the body — it's always the caller's own household, from `CurrentUser`.
#[derive(Debug, Deserialize)]
pub struct NewHouseholdMember {
    pub user_id: i32,
    pub r#type: String,
}

/// Body for `PATCH /household-members/{id}`. Only the role can change — `household_id` and
/// `user_id` are fixed at creation; delete and recreate the membership to move it.
#[derive(Debug, Deserialize)]
pub struct HouseholdMemberPatch {
    pub r#type: Option<String>,
}

/// Optional query params for `GET /household-members`. Absent fields mean "no filter". Always
/// further scoped to the caller's own household — see `CurrentUser`.
#[derive(Debug, Deserialize)]
pub struct HouseholdMemberFilter {
    pub user_id: Option<i32>,
}
