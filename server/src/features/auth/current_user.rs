//! The `CurrentUser` extractor: require a live session on any route that takes it as a handler
//! parameter, and hand back the signed-in user plus their household membership (if they've
//! onboarded). This is what closes #65 — every route that accepts `CurrentUser` is auth-gated by
//! construction, since actix-web runs the extractor before the handler body.

use std::future::Future;
use std::pin::Pin;

use actix_web::dev::Payload;
use actix_web::error::InternalError;
use actix_web::http::StatusCode;
use actix_web::{web, FromRequest, HttpRequest, HttpResponse};
use log::error;
use sqlx::PgPool;
use unic_langid::LanguageIdentifier;

use super::model::Membership;
use super::{repository, session};
use crate::features::users::model::User;
use crate::shared::http_error::{error_response, internal_error_response};
use crate::shared::l10n::L10n;

/// The signed-in caller. `household` is `None` until they complete onboarding.
pub struct CurrentUser {
    pub user: User,
    pub household: Option<Membership>,
}

impl CurrentUser {
    /// The household this caller belongs to, or `None` if they haven't onboarded.
    pub fn household_id(&self) -> Option<i32> {
        self.household.as_ref().map(|m| m.household_id)
    }

    /// The `household_members` row this caller's writes should be attributed to, or `None` if
    /// they haven't onboarded.
    // Not read yet: wired into `transactions` scoping in a follow-up commit.
    #[allow(dead_code)]
    pub fn household_member_id(&self) -> Option<i32> {
        self.household.as_ref().map(|m| m.id)
    }

    /// The common guard every household-scoped route needs: the caller's household id, or a 403
    /// `auth-no-household` response if they haven't onboarded yet.
    pub fn require_household_id(
        &self,
        l10n: &L10n,
        locale: &LanguageIdentifier,
    ) -> Result<i32, HttpResponse> {
        self.household_id()
            .ok_or_else(|| error_response(l10n, locale, StatusCode::FORBIDDEN, "auth-no-household"))
    }
}

impl FromRequest for CurrentUser {
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let pool = req
                .app_data::<web::Data<PgPool>>()
                .expect("PgPool must be configured as app_data")
                .clone();
            let l10n = req
                .app_data::<web::Data<L10n>>()
                .expect("L10n must be configured as app_data")
                .clone();
            let locale = l10n.locale();

            let user = match session::current_user(&req, &pool).await {
                Ok(Some(user)) => user,
                Ok(None) => {
                    let response = error_response(
                        &l10n,
                        &locale,
                        StatusCode::UNAUTHORIZED,
                        "auth-not-signed-in",
                    );
                    return Err(InternalError::from_response("not signed in", response).into());
                }
                Err(e) => {
                    error!("failed to load session user error={e}");
                    let response = internal_error_response(&l10n, &locale);
                    return Err(InternalError::from_response(e, response).into());
                }
            };

            let household = match repository::get_membership(&pool, user.id).await {
                Ok(household) => household,
                Err(e) => {
                    error!(
                        "failed to load household membership user_id={} error={e}",
                        user.id
                    );
                    let response = internal_error_response(&l10n, &locale);
                    return Err(InternalError::from_response(e, response).into());
                }
            };

            Ok(CurrentUser { user, household })
        })
    }
}
