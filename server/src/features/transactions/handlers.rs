//! HTTP API for transactions: JSON CRUD backed by Postgres.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{NewTransaction, TransactionFilter, TransactionPatch};
use super::repository;
use crate::shared::http_error::{
    error_response_with_n, internal_error_response, is_foreign_key_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Transaction id path (`/transactions/{id}`)
#[derive(Deserialize)]
struct TransactionIdPath {
    id: u32,
}

/// `POST /transactions` — create a transaction.
async fn create_transaction(
    new_transaction: web::Json<NewTransaction>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    match repository::create(&pool, &new_transaction).await {
        Ok(transaction) => HttpResponse::Created()
            .insert_header(("Location", format!("/transactions/{}", transaction.id)))
            .json(transaction),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-not-found",
            new_transaction.category_id as u32,
        ),
        Err(e) => {
            error!("failed to create transaction error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /transactions` — list transactions, optionally filtered by `date`, `merchant`, and/or a
/// free-text `search` matched against merchant, category, and amount.
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
        Ok(None) => not_found_response(&l10n, &locale, "transaction-not-found", id),
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
        Ok(None) => not_found_response(&l10n, &locale, "transaction-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-not-found",
            patch.category_id.unwrap_or_default() as u32,
        ),
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
        Ok(false) => not_found_response(&l10n, &locale, "transaction-not-found", id),
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
