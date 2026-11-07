//! HTTP API for households: JSON CRUD backed by Postgres.
//!
//! A household has no fields of its own to create or patch — see `members` for the users in it
//! and their roles. Leaving one, deleting it, and changing its `join_code` live here.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use unic_langid::LanguageIdentifier;

use super::model::PublicHousehold;
use super::repository::{self, DeleteOutcome};
use crate::features::auth::CurrentUser;
use crate::features::members::repository::{self as members_repository, MemberOutcome};
use crate::features::members::require_manager;
use crate::shared::http_error::{error_response, internal_error_response, not_found_response};
use crate::shared::l10n::L10n;

/// Household id path (`/households/{id}`)
#[derive(Deserialize)]
struct HouseholdIdPath {
    id: u32,
}

/// `POST /households` — create a new, empty household.
async fn create_household(
    _current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
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

/// `GET /households/{id}` — fetch the caller's own household, without its `join_code` (see
/// `PublicHousehold`).
async fn get_household(
    path: web::Path<HouseholdIdPath>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    if let Err(response) = require_own_household(&current_user, id, &l10n, &locale) {
        return response;
    }

    match repository::get(&pool, id as i32).await {
        Ok(Some(household)) => HttpResponse::Ok().json(PublicHousehold::from(household)),
        Ok(None) => not_found_response(&l10n, &locale, "household-not-found", id),
        Err(e) => {
            error!("failed to get household id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// The caller's own household id, if it's `id` — or a 404 as if `id` didn't exist, matching every
/// other household-scoped resource in this API.
fn require_own_household(
    current_user: &CurrentUser,
    id: u32,
    l10n: &L10n,
    locale: &LanguageIdentifier,
) -> Result<i32, HttpResponse> {
    match current_user.household_id() {
        Some(household_id) if household_id == id as i32 => Ok(household_id),
        _ => Err(not_found_response(l10n, locale, "household-not-found", id)),
    }
}

/// `DELETE /households/{id}` — delete the caller's own household and everything in it, when
/// they're its manager and its only member. They're left with no household, so onboarding asks
/// them to create or join one again.
async fn delete_household(
    path: web::Path<HouseholdIdPath>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match require_own_household(&current_user, id, &l10n, &locale) {
        Ok(household_id) => household_id,
        Err(response) => return response,
    };
    if let Err(response) = require_manager(&current_user, &l10n, &locale) {
        return response;
    }

    match repository::delete_with_data(&pool, household_id, current_user.user.id).await {
        Ok(DeleteOutcome::Deleted) => HttpResponse::NoContent().finish(),
        Ok(DeleteOutcome::NotFound) => {
            not_found_response(&l10n, &locale, "household-not-found", id)
        }
        Ok(DeleteOutcome::HasOtherMembers) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "household-has-members",
        ),
        Err(e) => {
            error!("failed to delete household id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `POST /households/{id}/leave` — leave the caller's own household. What they created and own
/// stays in it (see `members::repository::leave`). Refused for its last manager while others
/// remain, and for its only member, who deletes the household instead.
async fn leave_household(
    path: web::Path<HouseholdIdPath>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match require_own_household(&current_user, id, &l10n, &locale) {
        Ok(household_id) => household_id,
        Err(response) => return response,
    };

    match members_repository::leave(&pool, household_id, current_user.user.id).await {
        Ok(MemberOutcome::Done(())) => HttpResponse::NoContent().finish(),
        Ok(MemberOutcome::NotFound) => {
            not_found_response(&l10n, &locale, "household-not-found", id)
        }
        Ok(MemberOutcome::LastManager) => {
            error_response(&l10n, &locale, StatusCode::CONFLICT, "member-last-manager")
        }
        Ok(MemberOutcome::Alone) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "household-leave-alone",
        ),
        Err(e) => {
            error!("failed to leave household id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `POST /households/{id}/join-code` — give the caller's own household a new `join_code`, so the
/// old one stops working. Managers only. Returns the household with its new code.
async fn regenerate_join_code(
    path: web::Path<HouseholdIdPath>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match require_own_household(&current_user, id, &l10n, &locale) {
        Ok(household_id) => household_id,
        Err(response) => return response,
    };
    if let Err(response) = require_manager(&current_user, &l10n, &locale) {
        return response;
    }

    match repository::regenerate_join_code(&pool, household_id).await {
        Ok(Some(household)) => HttpResponse::Ok().json(household),
        Ok(None) => not_found_response(&l10n, &locale, "household-not-found", id),
        Err(e) => {
            error!("failed to regenerate join code id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the households feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/households", web::post().to(create_household))
        .route("/households/{id}", web::get().to(get_household))
        .route("/households/{id}", web::delete().to(delete_household))
        .route("/households/{id}/leave", web::post().to(leave_household))
        .route(
            "/households/{id}/join-code",
            web::post().to(regenerate_join_code),
        );
}
