use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// The SQL column giving a user's `picture_url`, for any query on `users`: where to fetch their
/// picture, or NULL if they have none. The `v` changes with every new picture, so a browser never
/// shows a cached old one.
macro_rules! picture_url_column {
    () => {
        "(SELECT '/members/' || user_pictures.user_id || '/picture?v=' || \
         floor(extract(epoch FROM user_pictures.updated_at) * 1000)::bigint \
         FROM user_pictures WHERE user_pictures.user_id = users.id) AS picture_url"
    };
}
pub(crate) use picture_url_column;

/// A single row in the `users` table.
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
    /// Where to fetch their picture (see `picture_url_column`), or `None` if they have none.
    pub picture_url: Option<String>,
    /// Onboarding steps this user has finished (see `auth::model::ONBOARDING_STEPS`).
    pub onboarding_steps: Vec<String>,
    /// The household this user belongs to (at most one, ever), or `None` until onboarding.
    pub household_id: Option<i32>,
    /// `family_manager` or `family_member` — set whenever `household_id` is.
    pub household_role: Option<String>,
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
}
