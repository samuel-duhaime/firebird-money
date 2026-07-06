//! HTTP API for transactions: JSON CRUD backed by Postgres.

use actix_web::http::header::{ContentLanguage, LanguageTag, QualityItem};
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use unic_langid::LanguageIdentifier;

use super::model::{NewTransaction, TransactionFilter, TransactionPatch};
use super::repository;
use crate::shared::l10n::L10n;

/// Transaction id path (`/transactions/{id}`)
#[derive(Deserialize)]
struct TransactionIdPath {
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

/// 404 JSON body for an unknown transaction id.
fn not_found_response(l10n: &L10n, locale: &LanguageIdentifier, id: u32) -> HttpResponse {
    let error = l10n.format_with_n(locale, "transaction-not-found", id);
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

/// `POST /transactions` — create a transaction.
async fn create_transaction(
    new_transaction: web::Json<NewTransaction>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    match repository::create(&pool, &new_transaction).await {
        Ok(transaction) => HttpResponse::Created()
            .insert_header(("Location", format!("/transactions/{}", transaction.id)))
            .json(transaction),
        Err(e) => {
            error!("failed to create transaction error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// `GET /transactions` — list transactions, optionally filtered by `date` and/or `merchant`.
async fn list_transactions(
    filter: web::Query<TransactionFilter>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    match repository::list(&pool, &filter).await {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => {
            error!("failed to list transactions error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// `GET /transactions/{id}` — fetch a single transaction.
async fn get_transaction(
    path: web::Path<TransactionIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::get(&pool, i64::from(id)).await {
        Ok(Some(transaction)) => HttpResponse::Ok().json(transaction),
        Ok(None) => not_found_response(&l10n, &locale, id),
        Err(e) => {
            error!("failed to get transaction id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /transactions/{id}` — partially update a transaction; unset fields are left unchanged.
async fn update_transaction(
    path: web::Path<TransactionIdPath>,
    patch: web::Json<TransactionPatch>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::update(&pool, i64::from(id), &patch).await {
        Ok(Some(transaction)) => HttpResponse::Ok().json(transaction),
        Ok(None) => not_found_response(&l10n, &locale, id),
        Err(e) => {
            error!("failed to update transaction id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /transactions/{id}` — delete a transaction.
async fn delete_transaction(
    path: web::Path<TransactionIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::delete(&pool, i64::from(id)).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, id),
        Err(e) => {
            error!("failed to delete transaction id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the transactions feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/transactions", web::get().to(list_transactions))
        .route("/transactions", web::post().to(create_transaction))
        .route("/transactions/{id}", web::get().to(get_transaction))
        .route("/transactions/{id}", web::patch().to(update_transaction))
        .route("/transactions/{id}", web::delete().to(delete_transaction));
}
