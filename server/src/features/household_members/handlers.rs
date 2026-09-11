//! HTTP API for household members: JSON CRUD backed by Postgres. Connects a `User` to a
//! `Household` with a role. Every route is scoped to the caller's own household.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{HouseholdMemberFilter, HouseholdMemberPatch, NewHouseholdMember};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_check_violation,
    is_foreign_key_violation, is_unique_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Household member id path (`/household-members/{id}`)
#[derive(Deserialize)]
struct HouseholdMemberIdPath {
    id: u32,
}

/// `POST /household-members` — connect a user to the caller's own household with a role. Only an
/// existing `family_manager` of that household may do this; `household_id` is never read from the
/// body — it's always the caller's own.
async fn create_household_member(
    new_member: web::Json<NewHouseholdMember>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let is_manager = current_user
        .household
        .as_ref()
        .is_some_and(|m| m.r#type == "family_manager");
    if !is_manager {
        return error_response(
            &l10n,
            &locale,
            StatusCode::FORBIDDEN,
            "household-member-requires-manager",
        );
    }

    match repository::create(pool.get_ref(), household_id, &new_member).await {
        Ok(member) => HttpResponse::Created()
            .insert_header(("Location", format!("/household-members/{}", member.id)))
            .json(member),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "household-member-duplicate",
        ),
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "household-member-invalid-type",
        ),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "user-not-found",
            new_member.user_id as u32,
        ),
        Err(e) => {
            error!("failed to create household member error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /household-members` — list the caller's household's memberships, optionally filtered by
/// `user_id`.
async fn list_household_members(
    filter: web::Query<HouseholdMemberFilter>,
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
        Ok(members) => HttpResponse::Ok().json(members),
        Err(e) => {
            error!("failed to list household members error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /household-members/{id}` — fetch a single membership from the caller's own household.
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

/// `PATCH /household-members/{id}` — change a membership's role, within the caller's own
/// household.
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
            "household-member-invalid-type",
        ),
        Err(e) => {
            error!("failed to update household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /household-members/{id}` — remove a membership from the caller's own household.
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

    match repository::delete(&pool, household_id, id as i32).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "household-member-not-found", id),
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
            "/household-members",
            web::post().to(create_household_member),
        )
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
