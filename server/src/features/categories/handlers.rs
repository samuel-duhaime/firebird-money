//! HTTP API for categories: JSON CRUD backed by Postgres.

use actix_web::http::header::{ContentLanguage, LanguageTag, QualityItem};
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use unic_langid::LanguageIdentifier;

use super::model::{CategoryPatch, NewCategory};
use super::repository;
use crate::shared::l10n::L10n;

/// Category id path (`/categories/{id}`)
#[derive(Deserialize)]
struct CategoryIdPath {
    id: u32,
}

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

/// 404 JSON body for an unknown category id.
fn not_found_response(l10n: &L10n, locale: &LanguageIdentifier, id: u32) -> HttpResponse {
    let error = l10n.format_with_n(locale, "category-not-found", id);
    HttpResponse::NotFound()
        .insert_header(content_language(locale))
        .json(ErrorBody { error })
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

/// `GET /categories` — list all categories.
async fn list_categories(pool: web::Data<PgPool>, l10n: web::Data<L10n>) -> impl Responder {
    match repository::list(&pool).await {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => {
            error!("failed to list categories error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// `GET /categories/{id}` — fetch a single category.
async fn get_category(
    path: web::Path<CategoryIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::get(&pool, id as i32).await {
        Ok(Some(category)) => HttpResponse::Ok().json(category),
        Ok(None) => not_found_response(&l10n, &locale, id),
        Err(e) => {
            error!("failed to get category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /categories/{id}` — partially update a category; unset fields are left unchanged.
async fn update_category(
    path: web::Path<CategoryIdPath>,
    patch: web::Json<CategoryPatch>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::update(&pool, id as i32, &patch).await {
        Ok(Some(category)) => HttpResponse::Ok().json(category),
        Ok(None) => not_found_response(&l10n, &locale, id),
        Err(e) => {
            error!("failed to update category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /categories/{id}` — delete a category.
async fn delete_category(
    path: web::Path<CategoryIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::delete(&pool, id as i32).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, id),
        Err(e) => {
            error!("failed to delete category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the categories feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/categories", web::get().to(list_categories))
        .route("/categories", web::post().to(create_category))
        .route("/categories/{id}", web::get().to(get_category))
        .route("/categories/{id}", web::patch().to(update_category))
        .route("/categories/{id}", web::delete().to(delete_category));
}
