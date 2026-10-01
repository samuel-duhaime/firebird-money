//! HTTP API for settings: the signed-in user's own display preferences, backed by Postgres.

use actix_web::{web, HttpResponse, Responder};
use log::error;
use sqlx::PgPool;

use super::model::{Settings, SettingsPatch};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::internal_error_response;
use crate::shared::l10n::L10n;

/// `GET /settings` — the signed-in user's display preferences, defaulted to every column visible
/// if they've never saved any.
async fn get_settings(
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    match repository::get(&pool, current_user.user.id).await {
        Ok(Some(settings)) => HttpResponse::Ok().json(settings),
        Ok(None) => HttpResponse::Ok().json(Settings::defaults(current_user.user.id)),
        Err(e) => {
            error!(
                "failed to get settings user_id={} error={e}",
                current_user.user.id
            );
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /settings` — partially update the signed-in user's display preferences; unset fields are
/// left unchanged. Creates the row on the user's first write.
async fn update_settings(
    patch: web::Json<SettingsPatch>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    match repository::update(&pool, current_user.user.id, &patch).await {
        Ok(settings) => HttpResponse::Ok().json(settings),
        Err(e) => {
            error!(
                "failed to update settings user_id={} error={e}",
                current_user.user.id
            );
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the settings feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/settings", web::get().to(get_settings))
        .route("/settings", web::patch().to(update_settings));
}
