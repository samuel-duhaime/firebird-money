//! HTTP API for categories: JSON CRUD backed by Postgres, scoped to the caller's household.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{CategoryPatch, NewCategory};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_foreign_key_violation,
    is_unique_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Category id path (`/categories/{id}`)
#[derive(Deserialize)]
struct CategoryIdPath {
    id: u32,
}

/// `POST /categories` — create a category in the caller's household.
async fn create_category(
    new_category: web::Json<NewCategory>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::create(&pool, household_id, &new_category).await {
        Ok(category) => HttpResponse::Created()
            .insert_header(("Location", format!("/categories/{}", category.id)))
            .json(category),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "category-duplicate-name",
        ),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-group-not-found",
            new_category.group_id as u32,
        ),
        Err(e) => {
            error!("failed to create category error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /categories` — list the caller's household's categories.
async fn list_categories(
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
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => {
            error!("failed to list categories error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /categories/{id}` — fetch a single category from the caller's household.
async fn get_category(
    path: web::Path<CategoryIdPath>,
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
        Ok(Some(category)) => HttpResponse::Ok().json(category),
        Ok(None) => not_found_response(&l10n, &locale, "category-not-found", id),
        Err(e) => {
            error!("failed to get category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /categories/{id}` — partially update a category in the caller's household; unset fields
/// are left unchanged.
async fn update_category(
    path: web::Path<CategoryIdPath>,
    patch: web::Json<CategoryPatch>,
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
        Ok(Some(category)) => HttpResponse::Ok().json(category),
        Ok(None) => not_found_response(&l10n, &locale, "category-not-found", id),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "category-duplicate-name",
        ),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-group-not-found",
            patch.group_id.unwrap_or_default() as u32,
        ),
        Err(e) => {
            error!("failed to update category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /categories/{id}` — delete a category from the caller's household.
async fn delete_category(
    path: web::Path<CategoryIdPath>,
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
        Ok(false) => not_found_response(&l10n, &locale, "category-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => {
            error_response_with_n(&l10n, &locale, StatusCode::CONFLICT, "category-in-use", id)
        }
        Err(e) => {
            error!("failed to delete category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the categories feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/categories", web::get().to(list_categories))
        .route("/categories", web::post().to(create_category))
        .route("/categories/{id}", web::get().to(get_category))
        .route("/categories/{id}", web::patch().to(update_category))
        .route("/categories/{id}", web::delete().to(delete_category));
}
