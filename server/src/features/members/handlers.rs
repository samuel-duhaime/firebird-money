//! HTTP API for members: the users in the caller's own household, with their role.
//! Nobody is added here — people join through `POST /auth/onboarding` with the household's code.

use actix_multipart::form::bytes::Bytes as UploadBytes;
use actix_multipart::form::{MultipartForm, MultipartFormConfig};
use actix_multipart::MultipartError;
use actix_web::error::{InternalError, PayloadError};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;
use unic_langid::LanguageIdentifier;

use super::model::{picture_content_type, MemberPatch};
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

/// Multipart body for `PUT /members/{id}/picture`. The client shrinks a picture well below this
/// limit before sending it, so only a request that skipped the app ever reaches it.
#[derive(MultipartForm)]
struct PictureForm {
    #[multipart(limit = "2MB")]
    file: UploadBytes,
}

/// A 403 `member-requires-manager` response unless the caller is their household's
/// `family_manager` — changing roles and removing people is theirs to do.
fn require_manager(
    current_user: &CurrentUser,
    l10n: &L10n,
    locale: &LanguageIdentifier,
) -> Result<(), HttpResponse> {
    let is_manager = current_user
        .household
        .as_ref()
        .is_some_and(|m| m.role == "family_manager");
    if is_manager {
        Ok(())
    } else {
        Err(error_response(
            l10n,
            locale,
            StatusCode::FORBIDDEN,
            "member-requires-manager",
        ))
    }
}

/// A 403 `member-picture-forbidden` response unless the caller is the member `id` or their
/// household's `family_manager` — a picture is only theirs to change.
fn require_self_or_manager(
    current_user: &CurrentUser,
    id: i32,
    l10n: &L10n,
    locale: &LanguageIdentifier,
) -> Result<(), HttpResponse> {
    if current_user.user.id == id || require_manager(current_user, l10n, locale).is_ok() {
        Ok(())
    } else {
        Err(error_response(
            l10n,
            locale,
            StatusCode::FORBIDDEN,
            "member-picture-forbidden",
        ))
    }
}

/// Converts a multipart parsing/extraction failure into the same localized JSON error shape the
/// rest of this API uses, instead of actix-multipart's plain-text default.
fn picture_upload_error_handler(err: MultipartError, req: &HttpRequest) -> actix_web::Error {
    let response = match req.app_data::<web::Data<L10n>>() {
        Some(l10n) => {
            let locale = l10n.locale();
            let message_id = match &err {
                MultipartError::Payload(PayloadError::Overflow) | MultipartError::Field { .. } => {
                    "member-picture-too-large"
                }
                _ => "member-picture-invalid",
            };
            error_response(l10n, &locale, StatusCode::BAD_REQUEST, message_id)
        }
        None => HttpResponse::BadRequest().finish(),
    };
    InternalError::from_response(err, response).into()
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

/// `PATCH /members/{id}` — change a member's role, within the caller's own household. Managers
/// only.
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
    if let Err(response) = require_manager(&current_user, &l10n, &locale) {
        return response;
    }

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
/// account is kept. Managers only, and blocked while they still have transactions in the household
/// (#109).
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
    if let Err(response) = require_manager(&current_user, &l10n, &locale) {
        return response;
    }

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

/// `GET /members/{id}/picture` — a picture of someone in the caller's own household, as image bytes.
/// Cached for good: `picture_url` carries a new version whenever the picture changes.
async fn get_member_picture(
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

    match repository::get_picture(&pool, household_id, id as i32).await {
        Ok(Some(picture)) => HttpResponse::Ok()
            .content_type(picture.content_type)
            .insert_header(("X-Content-Type-Options", "nosniff"))
            .insert_header(("Cache-Control", "private, max-age=31536000, immutable"))
            .body(picture.data),
        Ok(None) => not_found_response(&l10n, &locale, "member-picture-not-found", id),
        Err(e) => {
            error!("failed to get member picture id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PUT /members/{id}/picture` — set or replace a member's picture (multipart: `file`, a PNG, JPEG,
/// or WebP up to 2 MB) and return the updated member. That member or a manager only.
async fn put_member_picture(
    path: web::Path<MemberIdPath>,
    MultipartForm(form): MultipartForm<PictureForm>,
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
    if let Err(response) = require_self_or_manager(&current_user, id as i32, &l10n, &locale) {
        return response;
    }
    let Some(content_type) = picture_content_type(&form.file.data) else {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "member-picture-invalid",
        );
    };

    match repository::set_picture(
        &pool,
        household_id,
        id as i32,
        content_type,
        &form.file.data,
    )
    .await
    {
        Ok(true) => member_response(&pool, household_id, id, &l10n, &locale).await,
        Ok(false) => not_found_response(&l10n, &locale, "member-not-found", id),
        Err(e) => {
            error!("failed to set member picture id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /members/{id}/picture` — remove a member's picture, if they have one, and return the
/// updated member. That member or a manager only.
async fn delete_member_picture(
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
    if let Err(response) = require_self_or_manager(&current_user, id as i32, &l10n, &locale) {
        return response;
    }

    match repository::delete_picture(&pool, household_id, id as i32).await {
        Ok(()) => member_response(&pool, household_id, id, &l10n, &locale).await,
        Err(e) => {
            error!("failed to delete member picture id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// The member `id` of `household_id` as a response: 200 with the member, or 404.
async fn member_response(
    pool: &PgPool,
    household_id: i32,
    id: u32,
    l10n: &L10n,
    locale: &LanguageIdentifier,
) -> HttpResponse {
    match repository::get(pool, household_id, id as i32).await {
        Ok(Some(member)) => HttpResponse::Ok().json(member),
        Ok(None) => not_found_response(l10n, locale, "member-not-found", id),
        Err(e) => {
            error!("failed to get household member id={id} error={e}");
            internal_error_response(l10n, locale)
        }
    }
}

/// Registers the members feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/members", web::get().to(list_members))
        .route("/members/{id}", web::get().to(get_member))
        .route("/members/{id}", web::patch().to(update_member))
        .route("/members/{id}", web::delete().to(delete_member))
        // On the resource, so it doesn't replace the statement import's own `MultipartFormConfig`.
        .service(
            web::resource("/members/{id}/picture")
                .app_data(
                    MultipartFormConfig::default().error_handler(picture_upload_error_handler),
                )
                .route(web::get().to(get_member_picture))
                .route(web::put().to(put_member_picture))
                .route(web::delete().to(delete_member_picture)),
        );
}
