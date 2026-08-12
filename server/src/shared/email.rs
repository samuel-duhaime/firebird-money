//! Sends transactional email through [Resend](https://resend.com).
//!
//! Only outbound HTTPS is involved, so this works the same from localhost as from a real
//! deployment — no tunnel, no inbound webhook. Set `RESEND_API_KEY` and mail actually gets
//! delivered while you develop; leave `SKIP_EMAIL_VERIFICATION=true` and sign-in never gets here.

use log::error;
use serde::Serialize;

use super::config::AuthConfig;

const RESEND_SEND_URL: &str = "https://api.resend.com/emails";

/// Request body for Resend's send endpoint.
#[derive(Serialize)]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'a str,
    text: &'a str,
}

/// Sends the magic-link email. Errors are returned to the caller, which turns them into a 500 —
/// the login token stays unused, so the user can simply ask for another link.
pub async fn send_magic_link(
    config: &AuthConfig,
    client: &reqwest::Client,
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
        subject: "Your FireBird Money sign-in link",
        text: &format!(
            "Hi,\n\nClick the link below to sign in to FireBird Money. It expires shortly and can only be used once.\n\n{link}\n\nIf you didn't ask to sign in, you can ignore this email.\n"
        ),
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
        // detail is what makes a failed send diagnosable.
        let detail = response.text().await.unwrap_or_default();
        error!("resend rejected the magic-link email status={status} body={detail}");
        return Err(format!("Resend returned {status}"));
    }

    Ok(())
}
