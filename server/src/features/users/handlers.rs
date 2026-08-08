//! HTTP API for users: JSON CRUD backed by Postgres.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{NewUser, UserPatch};
use super::repository;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_check_violation,
    is_foreign_key_violation, is_unique_violation, not_found_response,
};
use crate::shared::l10n::L10n;

/// User id path (`/users/{id}`)
#[derive(Deserialize)]
struct UserIdPath {
    id: u32,
}

/// `POST /users` — create a user.
async fn create_user(
    new_user: web::Json<NewUser>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    match repository::create(&pool, &new_user).await {
        Ok(user) => HttpResponse::Created()
            .insert_header(("Location", format!("/users/{}", user.id)))
            .json(user),
        Err(e) if is_unique_violation(&e) => {
            error_response(&l10n, &locale, StatusCode::CONFLICT, "user-duplicate-email")
        }
        Err(e) => {
            error!("failed to create user error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /users/{id}` — fetch a single user.
async fn get_user(
    path: web::Path<UserIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::get(&pool, id as i32).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => not_found_response(&l10n, &locale, "user-not-found", id),
        Err(e) => {
            error!("failed to get user id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /users/{id}` — partially update a user; unset fields are left unchanged.
async fn update_user(
    path: web::Path<UserIdPath>,
    patch: web::Json<UserPatch>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::update(&pool, id as i32, &patch).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => not_found_response(&l10n, &locale, "user-not-found", id),
        Err(e) if is_unique_violation(&e) => {
            error_response(&l10n, &locale, StatusCode::CONFLICT, "user-duplicate-email")
        }
        Err(e) if is_check_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "user-invalid-status",
        ),
        Err(e) => {
            error!("failed to update user id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /users/{id}` — delete a user.
async fn delete_user(
    path: web::Path<UserIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::delete(&pool, id as i32).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "user-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => {
            error_response_with_n(&l10n, &locale, StatusCode::CONFLICT, "user-in-use", id)
        }
        Err(e) => {
            error!("failed to delete user id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the users feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/users", web::post().to(create_user))
        .route("/users/{id}", web::get().to(get_user))
        .route("/users/{id}", web::patch().to(update_user))
        .route("/users/{id}", web::delete().to(delete_user));
}
