//! HTTP API for households: JSON CRUD backed by Postgres.
//!
//! A household has no fields of its own to create or patch — see `household_members` for
//! connecting a user to one with a role.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::repository;
use crate::shared::http_error::{
    error_response_with_n, internal_error_response, is_foreign_key_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Household id path (`/households/{id}`)
#[derive(Deserialize)]
struct HouseholdIdPath {
    id: u32,
}

/// `POST /households` — create a new, empty household.
async fn create_household(pool: web::Data<PgPool>, l10n: web::Data<L10n>) -> impl Responder {
    match repository::create(&pool).await {
        Ok(household) => HttpResponse::Created()
            .insert_header(("Location", format!("/households/{}", household.id)))
            .json(household),
        Err(e) => {
            error!("failed to create household error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// `GET /households/{id}` — fetch a single household.
async fn get_household(
    path: web::Path<HouseholdIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::get(&pool, id as i32).await {
        Ok(Some(household)) => HttpResponse::Ok().json(household),
        Ok(None) => not_found_response(&l10n, &locale, "household-not-found", id),
        Err(e) => {
            error!("failed to get household id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /households/{id}` — delete a household.
async fn delete_household(
    path: web::Path<HouseholdIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::delete(&pool, id as i32).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "household-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => {
            error_response_with_n(&l10n, &locale, StatusCode::CONFLICT, "household-in-use", id)
        }
        Err(e) => {
            error!("failed to delete household id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the households feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/households", web::post().to(create_household))
        .route("/households/{id}", web::get().to(get_household))
        .route("/households/{id}", web::delete().to(delete_household));
}
