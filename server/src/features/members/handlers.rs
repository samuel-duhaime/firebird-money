//! HTTP API for members: the users in the caller's own household, with their role.
//! Nobody is added here — people join through `POST /auth/onboarding` with the household's code.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::MemberPatch;
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, internal_error_response, is_check_violation, is_foreign_key_violation,
    not_found_response,
};
use crate::shared::l10n::L10n;

/// Member id path (`/members/{id}`)
#[derive(Deserialize)]
struct MemberIdPath {
    id: u32,
}

/// `GET /members` — list the caller's household's members.
async fn list_members(
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

/// `GET /members/{id}` — fetch a single member of the caller's own household.
async fn get_member(
    path: web::Path<MemberIdPath>,
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
        Ok(None) => not_found_response(&l10n, &locale, "member-not-found", id),
        Err(e) => {
            error!("failed to get household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /members/{id}` — change a member's role, within the caller's own household.
async fn update_member(
    path: web::Path<MemberIdPath>,
    patch: web::Json<MemberPatch>,
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
        Ok(None) => not_found_response(&l10n, &locale, "member-not-found", id),
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "member-invalid-role",
        ),
        Err(e) => {
            error!("failed to update household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /members/{id}` — remove a member from the caller's own household. Their user
/// account is kept. Blocked while they still have transactions in the household (#109).
async fn delete_member(
    path: web::Path<MemberIdPath>,
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
        Ok(false) => not_found_response(&l10n, &locale, "member-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "member-has-transactions",
        ),
        Err(e) => {
            error!("failed to delete household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the members feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/members", web::get().to(list_members))
        .route("/members/{id}", web::get().to(get_member))
        .route("/members/{id}", web::patch().to(update_member))
        .route("/members/{id}", web::delete().to(delete_member));
}
