//! HTTP API for categories: JSON CRUD backed by Postgres.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{CategoryPatch, NewCategory};
use super::repository;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_check_violation,
    is_foreign_key_violation, is_unique_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Category id path (`/categories/{id}`)
#[derive(Deserialize)]
struct CategoryIdPath {
    id: u32,
}

/// `POST /categories` — create a category.
async fn create_category(
    new_category: web::Json<NewCategory>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    match repository::create(&pool, &new_category).await {
        Ok(category) => HttpResponse::Created()
            .insert_header(("Location", format!("/categories/{}", category.id)))
            .json(category),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "category-duplicate-name",
        ),
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-invalid-type",
        ),
        Err(e) => {
            error!("failed to create category error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /categories` — list all categories.
async fn list_categories(pool: web::Data<PgPool>, l10n: web::Data<L10n>) -> impl Responder {
    match repository::list(&pool).await {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => {
            error!("failed to list categories error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// `GET /categories/{id}` — fetch a single category.
async fn get_category(
    path: web::Path<CategoryIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::get(&pool, id as i32).await {
        Ok(Some(category)) => HttpResponse::Ok().json(category),
        Ok(None) => not_found_response(&l10n, &locale, "category-not-found", id),
        Err(e) => {
            error!("failed to get category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /categories/{id}` — partially update a category; unset fields are left unchanged.
async fn update_category(
    path: web::Path<CategoryIdPath>,
    patch: web::Json<CategoryPatch>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::update(&pool, id as i32, &patch).await {
        Ok(Some(category)) => HttpResponse::Ok().json(category),
        Ok(None) => not_found_response(&l10n, &locale, "category-not-found", id),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "category-duplicate-name",
        ),
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-invalid-type",
        ),
        Err(e) => {
            error!("failed to update category id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /categories/{id}` — delete a category.
async fn delete_category(
    path: web::Path<CategoryIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::delete(&pool, id as i32).await {
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
