//! HTTP API for merchants: JSON CRUD backed by Postgres, scoped to the caller's household (plus the
//! shared common set every household can see but never rename/delete — see `repository::update`/
//! `delete`).

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{MerchantFilter, MerchantPatch, NewMerchant};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_foreign_key_violation,
    is_unique_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Merchant id path (`/merchants/{id}`)
#[derive(Deserialize)]
struct MerchantIdPath {
    id: u32,
}

/// `POST /merchants` — create a custom merchant in the caller's household.
async fn create_merchant(
    new_merchant: web::Json<NewMerchant>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let name = new_merchant.name.trim();
    if name.is_empty() {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "merchant-name-required",
        );
    }

    match repository::create(
        &pool,
        household_id,
        &NewMerchant {
            name: name.to_string(),
        },
    )
    .await
    {
        Ok(merchant) => HttpResponse::Created()
            .insert_header(("Location", format!("/merchants/{}", merchant.id)))
            .json(merchant),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "merchant-duplicate-name",
        ),
        Err(e) => {
            error!("failed to create merchant error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /merchants` — list merchants visible to the caller's household (the shared common set plus
/// this household's own).
async fn list_merchants(
    filter: web::Query<MerchantFilter>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::list(&pool, household_id, &filter).await {
        Ok(merchants) => HttpResponse::Ok().json(merchants),
        Err(e) => {
            error!("failed to list merchants error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /merchants/{id}` — fetch a single merchant visible to the caller's household.
async fn get_merchant(
    path: web::Path<MerchantIdPath>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::get(&pool, household_id, id as i32).await {
        Ok(Some(merchant)) => HttpResponse::Ok().json(merchant),
        Ok(None) => not_found_response(&l10n, &locale, "merchant-not-found", id),
        Err(e) => {
            error!("failed to get merchant id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /merchants/{id}` — rename a merchant the caller's household owns; unset fields are left
/// unchanged. A common merchant (shared across every household) 404s here, same as any id outside
/// the caller's household — see `repository::update`.
async fn update_merchant(
    path: web::Path<MerchantIdPath>,
    patch: web::Json<MerchantPatch>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let name = match &patch.name {
        Some(name) if name.trim().is_empty() => {
            return error_response(
                &l10n,
                &locale,
                StatusCode::BAD_REQUEST,
                "merchant-name-required",
            );
        }
        Some(name) => Some(name.trim().to_string()),
        None => None,
    };

    match repository::update(&pool, household_id, id as i32, &MerchantPatch { name }).await {
        Ok(Some(merchant)) => HttpResponse::Ok().json(merchant),
        Ok(None) => not_found_response(&l10n, &locale, "merchant-not-found", id),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "merchant-duplicate-name",
        ),
        Err(e) => {
            error!("failed to update merchant id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /merchants/{id}` — delete a merchant the caller's household owns. Fails while any
/// transaction still carries it; a common merchant 404s here, same as `update_merchant`.
async fn delete_merchant(
    path: web::Path<MerchantIdPath>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::delete(&pool, household_id, id as i32).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "merchant-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => {
            error_response_with_n(&l10n, &locale, StatusCode::CONFLICT, "merchant-in-use", id)
        }
        Err(e) => {
            error!("failed to delete merchant id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the merchants feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/merchants", web::get().to(list_merchants))
        .route("/merchants", web::post().to(create_merchant))
        .route("/merchants/{id}", web::get().to(get_merchant))
        .route("/merchants/{id}", web::patch().to(update_merchant))
        .route("/merchants/{id}", web::delete().to(delete_merchant));
}
