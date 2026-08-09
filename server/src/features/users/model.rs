use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A single row in the `users` table. How a user relates to a household lives in
/// `household_members`, not here — a user can belong to more than one.
#[derive(Debug, Serialize, FromRow)]
pub struct User {
    pub id: i32,
    pub email: String,
    // Internal only; read starts with Google auth (#26).
    #[allow(dead_code)]
    #[serde(skip_serializing)]
    pub google_id: Option<String>,
    pub status: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Body for `POST /users`. `id`, `status`, and `created_at` are generated; `status` starts at
/// `pending`.
#[derive(Debug, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub google_id: Option<String>,
}

/// Body for `PATCH /users/{id}`. `None` fields are left unchanged.
#[derive(Debug, Deserialize)]
pub struct UserPatch {
    pub email: Option<String>,
    pub google_id: Option<String>,
    pub status: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
}
