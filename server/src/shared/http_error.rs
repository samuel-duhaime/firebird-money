//! Shared localized JSON error responses, used by every feature's handlers.

use actix_web::http::header::{ContentLanguage, LanguageTag, QualityItem};
use actix_web::http::StatusCode;
use actix_web::HttpResponse;
use serde::Serialize;
use unic_langid::LanguageIdentifier;

use super::l10n::L10n;

/// JSON body for error responses.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

/// `ContentLanguage` header value matching the active locale (falls back to English).
pub fn content_language(locale: &LanguageIdentifier) -> ContentLanguage {
    let lang_tag = locale.to_string();
    ContentLanguage(vec![QualityItem::max(
        LanguageTag::parse(&lang_tag).unwrap_or_else(|_| LanguageTag::parse("en").unwrap()),
    )])
}

/// Builds a localized JSON error response for the given status and Fluent message id.
pub fn error_response(
    l10n: &L10n,
    locale: &LanguageIdentifier,
    status: StatusCode,
    message_id: &str,
) -> HttpResponse {
    let error = l10n.format_message(locale, message_id, None);
    HttpResponse::build(status)
        .insert_header(content_language(locale))
        .json(ErrorBody { error })
}

/// Builds a localized JSON error response whose message takes a single numeric argument (e.g. an id).
pub fn error_response_with_n(
    l10n: &L10n,
    locale: &LanguageIdentifier,
    status: StatusCode,
    message_id: &str,
    n: u32,
) -> HttpResponse {
    let error = l10n.format_with_n(locale, message_id, n);
    HttpResponse::build(status)
        .insert_header(content_language(locale))
        .json(ErrorBody { error })
}

/// 404 JSON body for an unknown id, formatted with the numeric id.
pub fn not_found_response(
    l10n: &L10n,
    locale: &LanguageIdentifier,
    message_id: &str,
    id: u32,
) -> HttpResponse {
    error_response_with_n(l10n, locale, StatusCode::NOT_FOUND, message_id, id)
}

/// 500 JSON body for an unrecognized database failure.
pub fn internal_error_response(l10n: &L10n, locale: &LanguageIdentifier) -> HttpResponse {
    error_response(l10n, locale, StatusCode::INTERNAL_SERVER_ERROR, "internal-db-error")
}

/// `true` if `err` is a unique constraint violation (SQLSTATE 23505), e.g. a duplicate name.
pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error().is_some_and(|e| e.is_unique_violation())
}

/// `true` if `err` is a check constraint violation (SQLSTATE 23514), e.g. an invalid `type` value.
pub fn is_check_violation(err: &sqlx::Error) -> bool {
    err.as_database_error().is_some_and(|e| e.is_check_violation())
}

/// `true` if `err` is a foreign key violation (SQLSTATE 23503), e.g. an unknown `category_id`, or
/// deleting a row that's still referenced.
pub fn is_foreign_key_violation(err: &sqlx::Error) -> bool {
    err.as_database_error().is_some_and(|e| e.is_foreign_key_violation())
}
