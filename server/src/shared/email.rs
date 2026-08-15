//! Sends transactional email through [Resend](https://resend.com).
//!
//! Only outbound HTTPS is involved, so this works the same from localhost as from a real
//! deployment — no tunnel, no inbound webhook. Set `RESEND_API_KEY` and mail actually gets
//! delivered while you develop; leave `SKIP_EMAIL_VERIFICATION=true` and sign-in never gets here.
//!
//! Wording lives in `locales/*.ftl` like every other user-visible string, so emails go out in the
//! recipient's language.

use log::error;
use serde::Serialize;
use unic_langid::LanguageIdentifier;

use super::config::AuthConfig;
use super::l10n::L10n;

const RESEND_SEND_URL: &str = "https://api.resend.com/emails";

/// Request body for Resend's send endpoint.
#[derive(Serialize)]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'a str,
    text: &'a str,
}

/// Sends the magic-link email in `locale`. Errors are returned to the caller, which turns them
/// into a 502 — the login token stays unused, so the user can simply ask for another link.
pub async fn send_magic_link(
    config: &AuthConfig,
    client: &reqwest::Client,
    l10n: &L10n,
    locale: &LanguageIdentifier,
    to: &str,
    link: &str,
) -> Result<(), String> {
    let api_key = config
        .resend_api_key
        .as_deref()
        .ok_or_else(|| "RESEND_API_KEY is not set".to_string())?;

    let body = SendEmailRequest {
        from: &config.email_from,
        to: [to],
        subject: &l10n.format_message(locale, "auth-email-subject", None),
        text: &magic_link_text(l10n, locale, link),
    };

    let response = client
        .post(RESEND_SEND_URL)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("request to Resend failed: {e}"))?;

    let status = response.status();
    if !status.is_success() {
        // Resend explains refusals (unverified sender, invalid recipient) in the body; that
        // detail is what makes a failed send diagnosable. Resend sometimes echoes the recipient
        // address back in that detail (e.g. the test-domain restriction message), so it's
        // redacted before it reaches the log.
        let detail = response.text().await.unwrap_or_default();
        error!(
            "resend rejected the magic-link email status={status} body={}",
            redact_emails(&detail)
        );
        return Err(format!("Resend returned {status}"));
    }

    Ok(())
}

/// Replaces anything that looks like an email address with `[redacted]`, so a log line built
/// from third-party text (a Resend error body) can't leak who it was about.
fn redact_emails(text: &str) -> String {
    text.split(' ')
        .map(|word| {
            if word.contains('@') {
                "[redacted]"
            } else {
                word
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Builds the plain-text body: greeting, instructions, the link on its own line, sign-off.
///
/// Assembled here rather than as one Fluent message so translators get short sentences and the
/// layout stays in one place.
fn magic_link_text(l10n: &L10n, locale: &LanguageIdentifier, link: &str) -> String {
    let greeting = l10n.format_message(locale, "auth-email-greeting", None);
    let instructions = l10n.format_message(locale, "auth-email-instructions", None);
    let ignore = l10n.format_message(locale, "auth-email-ignore", None);

    format!("{greeting}\n\n{instructions}\n\n{link}\n\n{ignore}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_the_body_in_english() {
        let l10n = L10n::new();
        let text = magic_link_text(&l10n, &"en".parse().unwrap(), "https://example.com/link");

        assert!(text.starts_with("Hi,"));
        assert!(text.contains("https://example.com/link"));
        assert!(text.contains("expires in 15 minutes"));
    }

    #[test]
    fn builds_the_body_in_french() {
        let l10n = L10n::new();
        let text = magic_link_text(&l10n, &"fr".parse().unwrap(), "https://example.com/link");

        assert!(text.starts_with("Bonjour,"));
        assert!(text.contains("https://example.com/link"));
        assert!(text.contains("15 minutes"));
    }

    #[test]
    fn redact_emails_masks_addresses_but_keeps_the_rest() {
        let text = "You can only send testing emails to sam@example.com (validation_error)";
        let redacted = redact_emails(text);

        assert!(!redacted.contains('@'));
        assert!(redacted.contains("validation_error"));
        assert!(redacted.starts_with("You can only send testing emails to [redacted]"));
    }

    #[test]
    fn subjects_differ_between_locales() {
        let l10n = L10n::new();
        let english = l10n.format_message(&"en".parse().unwrap(), "auth-email-subject", None);
        let french = l10n.format_message(&"fr".parse().unwrap(), "auth-email-subject", None);

        assert_ne!(english, french);
        assert!(!english.is_empty());
    }
}
