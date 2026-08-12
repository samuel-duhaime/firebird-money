//! Reading and writing the session cookie.
//!
//! `#65` will turn this into a proper `CurrentUser` extractor applied to every route; for now the
//! auth handlers call it directly, since they're the only routes that know about sessions.

use actix_web::cookie::{Cookie, SameSite};
use actix_web::HttpRequest;
use sqlx::PgPool;

use super::{repository, tokens};
use crate::features::users::model::User;
use crate::shared::config::{AuthConfig, SESSION_COOKIE_NAME, SESSION_TTL_DAYS};

/// The raw session token carried by the request's cookie, if it has one.
pub fn session_token(req: &HttpRequest) -> Option<String> {
    req.cookie(SESSION_COOKIE_NAME)
        .map(|cookie| cookie.value().to_string())
}

/// Resolves the request's session cookie to the signed-in user, or `None` when there's no cookie
/// or it no longer matches a live session.
pub async fn current_user(req: &HttpRequest, pool: &PgPool) -> Result<Option<User>, sqlx::Error> {
    let Some(token) = session_token(req) else {
        return Ok(None);
    };

    repository::get_session_user(pool, &tokens::hash(&token)).await
}

/// Builds the cookie that keeps the user signed in.
///
/// `Secure` is on only in production: localhost is plain HTTP, and a `Secure` cookie there would
/// simply never be stored, breaking dev sign-in.
pub fn build_session_cookie(config: &AuthConfig, raw_token: String) -> Cookie<'static> {
    Cookie::build(SESSION_COOKIE_NAME, raw_token)
        .path("/")
        .http_only(true)
        .secure(config.is_production())
        .same_site(SameSite::Lax)
        .max_age(actix_web::cookie::time::Duration::days(SESSION_TTL_DAYS))
        .finish()
}

/// Builds the cookie that clears the session on logout.
pub fn build_removal_cookie(config: &AuthConfig) -> Cookie<'static> {
    let mut cookie = build_session_cookie(config, String::new());
    cookie.make_removal();
    cookie
}
