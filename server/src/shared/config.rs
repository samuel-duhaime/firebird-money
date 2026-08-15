//! Auth and email settings read from the environment (see `.env.example`).
//!
//! Everything here is validated once at startup so a misconfigured server refuses to boot instead
//! of failing later, on someone's sign-in attempt.

/// `APP_ENV` values (case and surrounding whitespace insensitive, so a stray "Production" in a
/// real deployment still gets production's protections). Nothing else is accepted — see
/// `validate_app_env` — so a typo or a forgotten variable fails startup instead of silently
/// landing on development's weaker guarantees.
const PRODUCTION_APP_ENV: &str = "production";
const DEVELOPMENT_APP_ENV: &str = "development";

/// Whether an `APP_ENV` value means production. Only called after `validate_app_env` has already
/// confirmed the value is one of the two recognized ones.
fn is_production_env(app_env: &str) -> bool {
    app_env.trim().eq_ignore_ascii_case(PRODUCTION_APP_ENV)
}

/// How long a magic link stays valid.
pub const LOGIN_TOKEN_TTL_MINUTES: i64 = 15;

/// How long a session cookie stays valid before the user has to sign in again.
pub const SESSION_TTL_DAYS: i64 = 30;

/// Name of the session cookie issued on a successful login.
pub const SESSION_COOKIE_NAME: &str = "session";

/// Resolved auth/email configuration, built once in `main` and shared with the handlers.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Raw `APP_ENV` value. Read it through `is_production`, which handles case and whitespace.
    pub app_env: String,

    /// When true, sign-in skips the email entirely and logs the user straight in. Dev only — the
    /// startup guard refuses this in production.
    pub skip_email_verification: bool,

    /// Resend API key. Always present when `skip_email_verification` is false.
    pub resend_api_key: Option<String>,

    /// `From` header of the magic-link emails.
    pub email_from: String,

    /// Client origin the magic link points at.
    pub client_base_url: String,
}

impl AuthConfig {
    /// Reads the configuration from the process environment and validates it.
    pub fn from_env() -> Result<Self, String> {
        let app_env = std::env::var("APP_ENV").unwrap_or_default();
        let skip_email_verification = env_flag("SKIP_EMAIL_VERIFICATION");
        let resend_api_key = non_empty_var("RESEND_API_KEY");

        validate_app_env(&app_env)?;
        validate_email_verification(&app_env, skip_email_verification)?;
        validate_email_provider(skip_email_verification, resend_api_key.as_deref())?;

        Ok(Self {
            app_env,
            skip_email_verification,
            resend_api_key,
            email_from: non_empty_var("EMAIL_FROM")
                .unwrap_or_else(|| "FireBird Money <onboarding@resend.dev>".to_string()),
            client_base_url: non_empty_var("CLIENT_BASE_URL")
                .unwrap_or_else(|| "http://localhost:5173".to_string()),
        })
    }

    /// Whether this process is running as production. Drives the `Secure` flag on the session
    /// cookie, which can't be set on localhost's plain HTTP.
    pub fn is_production(&self) -> bool {
        is_production_env(&self.app_env)
    }
}

/// Reads a boolean-ish env var. Only "true" and "1" count as true.
fn env_flag(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            let value = value.trim().to_ascii_lowercase();
            value == "true" || value == "1"
        })
        .unwrap_or(false)
}

/// Reads an env var, treating blank as unset.
fn non_empty_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Rejects an `APP_ENV` that isn't exactly `"production"` or `"development"`. Without this, a
/// missing variable or a typo (`"prod"`) silently gets treated as development: `validate_email_verification`
/// would then allow `SKIP_EMAIL_VERIFICATION=true`, and `is_production` would tell the session
/// cookie to skip `Secure`. Both are fine in dev and dangerous in a real deployment.
fn validate_app_env(app_env: &str) -> Result<(), String> {
    let value = app_env.trim();
    if value.eq_ignore_ascii_case(PRODUCTION_APP_ENV)
        || value.eq_ignore_ascii_case(DEVELOPMENT_APP_ENV)
    {
        return Ok(());
    }
    Err(format!(
        "APP_ENV must be \"{PRODUCTION_APP_ENV}\" or \"{DEVELOPMENT_APP_ENV}\", got {app_env:?}"
    ))
}

/// The safeguard: the "skip the email, just log me in" shortcut must never be live in production.
///
/// Kept as a pure function of its two inputs so it can be tested without mutating the process
/// environment, which is global (and `unsafe`) in recent Rust editions.
fn validate_email_verification(app_env: &str, skip_email_verification: bool) -> Result<(), String> {
    if is_production_env(app_env) && skip_email_verification {
        return Err(format!(
            "SKIP_EMAIL_VERIFICATION must not be true when APP_ENV is \"{PRODUCTION_APP_ENV}\": it would let anyone sign in as any email without proving they own it"
        ));
    }
    Ok(())
}

/// Sending a real email needs a real API key — catch the missing one at startup rather than on a
/// user's first sign-in attempt.
fn validate_email_provider(
    skip_email_verification: bool,
    resend_api_key: Option<&str>,
) -> Result<(), String> {
    if !skip_email_verification && resend_api_key.is_none() {
        return Err(
            "RESEND_API_KEY must be set unless SKIP_EMAIL_VERIFICATION is true: there is no way to send the magic-link email".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_skipping_email_verification_in_production() {
        let result = validate_email_verification(PRODUCTION_APP_ENV, true);
        assert!(result.is_err());
    }

    #[test]
    fn allows_email_verification_in_production() {
        assert!(validate_email_verification(PRODUCTION_APP_ENV, false).is_ok());
    }

    #[test]
    fn allows_skipping_email_verification_outside_production() {
        assert!(validate_email_verification("development", true).is_ok());
        assert!(validate_email_verification("", true).is_ok());
    }

    #[test]
    fn production_check_ignores_case_and_whitespace() {
        assert!(validate_email_verification("Production", true).is_err());
        assert!(validate_email_verification("  PRODUCTION  ", true).is_err());
    }

    #[test]
    fn rejects_unset_or_unrecognized_app_env() {
        // A forgotten variable or a typo like "prod" no longer falls through to development —
        // `from_env` refuses to start instead.
        assert!(validate_app_env("").is_err());
        assert!(validate_app_env("prod").is_err());
        assert!(validate_app_env("staging").is_err());
    }

    #[test]
    fn accepts_the_two_recognized_app_env_values_regardless_of_case() {
        assert!(validate_app_env("production").is_ok());
        assert!(validate_app_env("Production").is_ok());
        assert!(validate_app_env("development").is_ok());
        assert!(validate_app_env("  Development  ").is_ok());
    }

    #[test]
    fn requires_an_api_key_when_sending_real_emails() {
        assert!(validate_email_provider(false, None).is_err());
        assert!(validate_email_provider(false, Some("re_123")).is_ok());
    }

    #[test]
    fn allows_a_missing_api_key_when_email_is_skipped() {
        assert!(validate_email_provider(true, None).is_ok());
    }
}
