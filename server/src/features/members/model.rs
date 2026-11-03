use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// A user seen as a member of their household — what `/members` returns, so the members
/// page can show each person's role, name, email, and account status. `id` is the user's id.
#[derive(Debug, Serialize, FromRow)]
pub struct Member {
    pub id: i32,
    pub household_id: i32,
    pub role: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    /// Where to fetch their picture (`GET /members/{id}/picture`), or `None` if they have none.
    pub picture_url: Option<String>,
}

/// Body for `PATCH /members/{id}`. Only the role can change.
#[derive(Debug, Deserialize)]
pub struct MemberPatch {
    pub role: Option<String>,
}

/// A member's stored picture, as served by `GET /members/{id}/picture`.
#[derive(Debug, FromRow)]
pub struct MemberPicture {
    pub content_type: String,
    pub data: Vec<u8>,
}

/// The image type `bytes` really are, whatever the upload claimed, or `None` if they aren't one of
/// the types we accept. SVG is left out on purpose: it can carry scripts.
pub fn picture_content_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.starts_with(b"\xFF\xD8\xFF") {
        Some("image/jpeg")
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}
