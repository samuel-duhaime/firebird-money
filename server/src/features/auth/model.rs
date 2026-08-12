use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::features::users::model::User;

/// Body for `POST /auth/request-login`.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,

    /// Language the magic-link email should use (`en` or `fr`), i.e. whatever the client's UI is
    /// showing. Falls back to the server's `DEFAULT_LANGUAGE`.
    pub language: Option<String>,
}

/// Query for `GET /auth/verify` — the token carried by the magic link.
#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    pub token: String,
}

/// Body for `POST /auth/onboarding`. A `join_code` joins that household as a `family_member`;
/// without one, a brand new household is created and the caller becomes its `family_manager`.
#[derive(Debug, Deserialize)]
pub struct OnboardingRequest {
    pub join_code: Option<String>,
}

/// One household the current user belongs to, with the role they hold in it.
#[derive(Debug, Serialize, FromRow)]
pub struct Membership {
    pub household_id: i32,
    pub join_code: String,
    pub r#type: String,
}

/// Who the caller is: the user plus every household they belong to. The payload behind
/// `GET /auth/me`, and what a successful login returns so the client doesn't need a second call.
#[derive(Debug, Serialize)]
pub struct CurrentUser {
    pub user: User,
    pub households: Vec<Membership>,
}

/// Response to `POST /auth/request-login`.
#[derive(Debug, Serialize)]
pub struct RequestLoginResponse {
    /// `email_sent` when a magic link went out, `signed_in` when `SKIP_EMAIL_VERIFICATION` logged
    /// the caller straight in.
    pub status: &'static str,

    /// Present only for `signed_in`, so the client can skip the `GET /auth/me` round trip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<CurrentUser>,
}

/// Trims and lowercases an email, returning `None` if it can't plausibly be one.
///
/// Deliberately loose: the magic link is the real check, since only the mailbox owner can click
/// it. This just catches obvious typos before we create a `User` row for them.
pub fn normalize_email(raw: &str) -> Option<String> {
    let email = raw.trim().to_lowercase();

    let (local, domain) = email.split_once('@')?;
    let plausible = !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.contains(char::is_whitespace);

    plausible.then_some(email)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_and_lowercases() {
        assert_eq!(
            normalize_email("  Sam@Example.COM "),
            Some("sam@example.com".to_string())
        );
    }

    #[test]
    fn rejects_implausible_addresses() {
        for raw in [
            "",
            "sam",
            "sam@",
            "@example.com",
            "sam@example",
            "a b@c.com",
        ] {
            assert_eq!(normalize_email(raw), None, "should reject {raw:?}");
        }
    }
}
