//! HTTP API for household members: JSON CRUD backed by Postgres. Connects a `User` to a
//! `Household` with a role.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{HouseholdMemberFilter, HouseholdMemberPatch, NewHouseholdMember};
use super::repository;
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

/// Maps a foreign key violation on `household_members` to a `BAD_REQUEST` naming whichever
/// referenced id (`household_id` or `user_id`) doesn't exist.
fn foreign_key_error_response(
    l10n: &L10n,
    locale: &unic_langid::LanguageIdentifier,
    e: &sqlx::Error,
    new_member: &NewHouseholdMember,
) -> HttpResponse {
    let constraint = e
        .as_database_error()
        .and_then(|e| e.constraint())
        .unwrap_or_default();
    if constraint.contains("household_id") {
        error_response_with_n(
            l10n,
            locale,
            StatusCode::BAD_REQUEST,
            "household-not-found",
            new_member.household_id as u32,
        )
    } else {
        error_response_with_n(
            l10n,
            locale,
            StatusCode::BAD_REQUEST,
            "user-not-found",
            new_member.user_id as u32,
        )
    }
}

/// `POST /household-members` — connect a user to a household with a role.
async fn create_household_member(
    new_member: web::Json<NewHouseholdMember>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    match repository::create(&pool, &new_member).await {
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
        Err(e) if is_foreign_key_violation(&e) => {
            foreign_key_error_response(&l10n, &locale, &e, &new_member)
        }
        Err(e) => {
            error!("failed to create household member error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /household-members` — list memberships, optionally filtered by `household_id` and/or
/// `user_id`.
async fn list_household_members(
    filter: web::Query<HouseholdMemberFilter>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    match repository::list(&pool, &filter).await {
        Ok(members) => HttpResponse::Ok().json(members),
        Err(e) => {
            error!("failed to list household members error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// `GET /household-members/{id}` — fetch a single membership.
async fn get_household_member(
    path: web::Path<HouseholdMemberIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::get(&pool, id as i32).await {
        Ok(Some(member)) => HttpResponse::Ok().json(member),
        Ok(None) => not_found_response(&l10n, &locale, "household-member-not-found", id),
        Err(e) => {
            error!("failed to get household member id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /household-members/{id}` — change a membership's role.
async fn update_household_member(
    path: web::Path<HouseholdMemberIdPath>,
    patch: web::Json<HouseholdMemberPatch>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::update(&pool, id as i32, &patch).await {
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

/// `DELETE /household-members/{id}` — remove a membership.
async fn delete_household_member(
    path: web::Path<HouseholdMemberIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::delete(&pool, id as i32).await {
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
