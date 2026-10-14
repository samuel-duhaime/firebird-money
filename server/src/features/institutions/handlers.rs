//! HTTP API for institutions: JSON CRUD backed by Postgres, scoped to the caller's household (plus
//! the shared common set every household can see but never rename/delete — see
//! `repository::update`/`delete`).

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{InstitutionPatch, NewInstitution};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_foreign_key_violation,
    is_unique_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Institution id path (`/institutions/{id}`)
#[derive(Deserialize)]
struct InstitutionIdPath {
    id: u32,
}

/// Why a submitted `website_url` was rejected.
struct InvalidWebsiteUrl;

/// Trims a submitted `website_url`: blank becomes `Some("")` (clear it — `create_institution` turns
/// that into `None`), anything else must be an `http(s)://` URL. The client renders it as a link, so
/// this also keeps out `javascript:` and friends.
fn normalize_website_url(url: &str) -> Result<String, InvalidWebsiteUrl> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let lower = trimmed.to_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        Ok(trimmed.to_string())
    } else {
        Err(InvalidWebsiteUrl)
    }
}

/// `POST /institutions` — create a custom institution in the caller's household.
async fn create_institution(
    new_institution: web::Json<NewInstitution>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let name = new_institution.name.trim();
    if name.is_empty() {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "institution-name-required",
        );
    }

    let website_url = match new_institution
        .website_url
        .as_deref()
        .map(normalize_website_url)
    {
        Some(Ok(url)) if url.is_empty() => None,
        Some(Ok(url)) => Some(url),
        Some(Err(InvalidWebsiteUrl)) => {
            return error_response(
                &l10n,
                &locale,
                StatusCode::BAD_REQUEST,
                "institution-invalid-website",
            );
        }
        None => None,
    };

    match repository::create(
        &pool,
        household_id,
        &NewInstitution {
            name: name.to_string(),
            website_url,
        },
    )
    .await
    {
        Ok(institution) => HttpResponse::Created()
            .insert_header(("Location", format!("/institutions/{}", institution.id)))
            .json(institution),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "institution-duplicate-name",
        ),
        Err(e) => {
            error!("failed to create institution error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /institutions` — list institutions visible to the caller's household (the shared common
/// set plus this household's own), A-Z.
async fn list_institutions(
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
        Ok(institutions) => HttpResponse::Ok().json(institutions),
        Err(e) => {
            error!("failed to list institutions error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /institutions/{id}` — fetch a single institution visible to the caller's household.
async fn get_institution(
    path: web::Path<InstitutionIdPath>,
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
        Ok(Some(institution)) => HttpResponse::Ok().json(institution),
        Ok(None) => not_found_response(&l10n, &locale, "institution-not-found", id),
        Err(e) => {
            error!("failed to get institution id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /institutions/{id}` — rename (or change the website of) an institution the caller's
/// household owns. A common institution 404s here, same as any id outside the caller's household.
async fn update_institution(
    path: web::Path<InstitutionIdPath>,
    patch: web::Json<InstitutionPatch>,
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
                "institution-name-required",
            );
        }
        Some(name) => Some(name.trim().to_string()),
        None => None,
    };

    let website_url = match patch.website_url.as_deref().map(normalize_website_url) {
        Some(Ok(url)) => Some(url),
        Some(Err(InvalidWebsiteUrl)) => {
            return error_response(
                &l10n,
                &locale,
                StatusCode::BAD_REQUEST,
                "institution-invalid-website",
            );
        }
        None => None,
    };

    match repository::update(
        &pool,
        household_id,
        id as i32,
        &InstitutionPatch { name, website_url },
    )
    .await
    {
        Ok(Some(institution)) => HttpResponse::Ok().json(institution),
        Ok(None) => not_found_response(&l10n, &locale, "institution-not-found", id),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "institution-duplicate-name",
        ),
        Err(e) => {
            error!("failed to update institution id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /institutions/{id}` — delete an institution the caller's household owns. Fails while any
/// account is still held there; a common institution 404s, same as `update_institution`.
async fn delete_institution(
    path: web::Path<InstitutionIdPath>,
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
        Ok(false) => not_found_response(&l10n, &locale, "institution-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "institution-in-use",
            id,
        ),
        Err(e) => {
            error!("failed to delete institution id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the institutions feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/institutions", web::get().to(list_institutions))
        .route("/institutions", web::post().to(create_institution))
        .route("/institutions/{id}", web::get().to(get_institution))
        .route("/institutions/{id}", web::patch().to(update_institution))
        .route("/institutions/{id}", web::delete().to(delete_institution));
}
