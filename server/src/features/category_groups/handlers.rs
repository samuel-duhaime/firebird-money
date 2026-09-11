//! HTTP API for category groups: JSON CRUD backed by Postgres, scoped to the caller's household.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{CategoryGroupPatch, NewCategoryGroup};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_check_violation,
    is_foreign_key_violation, is_unique_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// Category group id path (`/category-groups/{id}`)
#[derive(Deserialize)]
struct CategoryGroupIdPath {
    id: u32,
}

/// `POST /category-groups` — create a category group in the caller's household.
async fn create_category_group(
    new_group: web::Json<NewCategoryGroup>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::create(&pool, household_id, &new_group).await {
        Ok(group) => HttpResponse::Created()
            .insert_header(("Location", format!("/category-groups/{}", group.id)))
            .json(group),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "category-group-duplicate-name",
        ),
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-group-invalid-type",
        ),
        Err(e) => {
            error!("failed to create category group error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /category-groups` — list the caller's household's category groups.
async fn list_category_groups(
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
        Ok(groups) => HttpResponse::Ok().json(groups),
        Err(e) => {
            error!("failed to list category groups error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /category-groups/{id}` — fetch a single category group from the caller's household.
async fn get_category_group(
    path: web::Path<CategoryGroupIdPath>,
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
        Ok(Some(group)) => HttpResponse::Ok().json(group),
        Ok(None) => not_found_response(&l10n, &locale, "category-group-not-found", id),
        Err(e) => {
            error!("failed to get category group id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /category-groups/{id}` — partially update a category group in the caller's household;
/// unset fields are left unchanged.
async fn update_category_group(
    path: web::Path<CategoryGroupIdPath>,
    patch: web::Json<CategoryGroupPatch>,
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
        Ok(Some(group)) => HttpResponse::Ok().json(group),
        Ok(None) => not_found_response(&l10n, &locale, "category-group-not-found", id),
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "category-group-duplicate-name",
        ),
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-group-invalid-type",
        ),
        Err(e) => {
            error!("failed to update category group id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /category-groups/{id}` — delete a category group from the caller's household. Fails
/// while it still has categories in it.
async fn delete_category_group(
    path: web::Path<CategoryGroupIdPath>,
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
        Ok(false) => not_found_response(&l10n, &locale, "category-group-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "category-group-in-use",
            id,
        ),
        Err(e) => {
            error!("failed to delete category group id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the category groups feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/category-groups", web::get().to(list_category_groups))
        .route("/category-groups", web::post().to(create_category_group))
        .route("/category-groups/{id}", web::get().to(get_category_group))
        .route(
            "/category-groups/{id}",
            web::patch().to(update_category_group),
        )
        .route(
            "/category-groups/{id}",
            web::delete().to(delete_category_group),
        );
}
