//! HTTP API for household members: the users in the caller's own household, with their role.
//! Nobody is added here — people join through `POST /auth/onboarding` with the household's code.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::HouseholdMemberPatch;
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, internal_error_response, is_check_violation, is_foreign_key_violation,
    not_found_response,
};
use crate::shared::l10n::L10n;

/// Household member id path (`/household-members/{id}`)
#[derive(Deserialize)]
struct HouseholdMemberIdPath {
    id: u32,
}

/// `GET /household-members` — list the caller's household's members.
async fn list_household_members(
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::list(&pool, household_id).await {
        Ok(members) => HttpResponse::Ok().json(members),
        Err(e) => {
            error!("failed to list household members error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /household-members/{id}` — fetch a single member of the caller's own household.
async fn get_household_member(
    path: web::Path<HouseholdMemberIdPath>,
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
        Ok(Some(member)) => HttpResponse::Ok().json(member),
        Ok(None) => not_found_response(&l10n, &locale, "household-member-not-found", id),
        Err(e) => {
            error!("failed to get household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /household-members/{id}` — change a member's role, within the caller's own household.
async fn update_household_member(
    path: web::Path<HouseholdMemberIdPath>,
    patch: web::Json<HouseholdMemberPatch>,
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

    match repository::update(&pool, household_id, id as i32, &patch).await {
        Ok(Some(member)) => HttpResponse::Ok().json(member),
        Ok(None) => not_found_response(&l10n, &locale, "household-member-not-found", id),
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "household-member-invalid-role",
        ),
        Err(e) => {
            error!("failed to update household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /household-members/{id}` — remove a member from the caller's own household. Their user
/// account is kept. Blocked while they still have transactions in the household (#109).
async fn delete_household_member(
    path: web::Path<HouseholdMemberIdPath>,
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

    match repository::remove(&pool, household_id, id as i32).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "household-member-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "household-member-has-transactions",
        ),
        Err(e) => {
            error!("failed to delete household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the household members feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/household-members", web::get().to(list_household_members))
        .route(
            "/household-members/{id}",
            web::get().to(get_household_member),
        )
        .route(
            "/household-members/{id}",
            web::patch().to(update_household_member),
        )
        .route(
            "/household-members/{id}",
            web::delete().to(delete_household_member),
        );
}
