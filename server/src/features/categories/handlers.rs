//! HTTP API for categories: JSON create backed by Postgres.

use actix_web::http::header::{ContentLanguage, LanguageTag, QualityItem};
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Serialize;
use sqlx::PgPool;
use unic_langid::LanguageIdentifier;

use super::model::NewCategory;
use super::repository;
use crate::shared::l10n::L10n;

/// JSON body for error responses.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

/// `ContentLanguage` header value matching the active locale (falls back to English).
fn content_language(locale: &LanguageIdentifier) -> ContentLanguage {
    let lang_tag = locale.to_string();
    ContentLanguage(vec![QualityItem::max(
        LanguageTag::parse(&lang_tag).unwrap_or_else(|_| LanguageTag::parse("en").unwrap()),
    )])
}

/// 500 JSON body for a database failure.
fn internal_error_response(l10n: &L10n, locale: &LanguageIdentifier) -> HttpResponse {
    let error = l10n.format_message(locale, "internal-db-error", None);
    HttpResponse::InternalServerError()
        .insert_header(content_language(locale))
        .json(ErrorBody { error })
}

/// `POST /categories` — create a category.
async fn create_category(
    new_category: web::Json<NewCategory>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    match repository::create(&pool, &new_category).await {
        Ok(category) => HttpResponse::Created()
            .insert_header(("Location", format!("/categories/{}", category.id)))
            .json(category),
        Err(e) => {
            error!("failed to create category error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// Registers the categories feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/categories", web::post().to(create_category));
}
