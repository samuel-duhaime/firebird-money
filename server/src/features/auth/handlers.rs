//! HTTP API for passwordless email sign-in and the household onboarding that follows a first
//! login.
//!
//! The flow: `POST /auth/request-login` mails a one-time link, `GET /auth/verify` spends it and
//! opens a session, `POST /auth/onboarding` puts the new user in a household. With
//! `SKIP_EMAIL_VERIFICATION=true` the first step spends its own token immediately and signs the
//! caller in, so localhost needs no mail provider and nobody has to dig a link out of a log.

use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::{Duration, Utc};
use log::error;
use sqlx::PgPool;

use super::model::{
    normalize_email, CurrentUser, LoginRequest, Membership, OnboardingRequest,
    RequestLoginResponse, VerifyQuery,
};
use super::session::{build_removal_cookie, build_session_cookie, current_user, session_token};
use super::{repository, tokens};
use crate::features::household_members::model::NewHouseholdMember;
use crate::features::household_members::repository as household_members_repository;
use crate::features::households::repository as households_repository;
use crate::features::users::model::User;
use crate::features::users::repository as users_repository;
use crate::shared::config::{AuthConfig, LOGIN_TOKEN_TTL_MINUTES, SESSION_TTL_DAYS};
use crate::shared::email;
use crate::shared::http_error::{error_response, internal_error_response, is_unique_violation};
use crate::shared::l10n::L10n;

/// `POST /auth/request-login` — find-or-create the user, issue a one-time token, and either mail
/// the magic link or (dev shortcut) spend the token right away.
async fn request_login(
    body: web::Json<LoginRequest>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
    config: web::Data<AuthConfig>,
    http_client: web::Data<reqwest::Client>,
) -> impl Responder {
    let locale = l10n.locale();

    let Some(email_address) = normalize_email(&body.email) else {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "auth-email-invalid",
        );
    };

    let user = match users_repository::find_or_create_by_email(&pool, &email_address).await {
        Ok(user) => user,
        Err(e) => {
            error!("failed to find or create user for sign-in error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };

    let raw_token = tokens::generate();
    let expires_at = Utc::now() + Duration::minutes(LOGIN_TOKEN_TTL_MINUTES);
    if let Err(e) =
        repository::create_login_token(&pool, user.id, &tokens::hash(&raw_token), expires_at).await
    {
        error!("failed to store login token user_id={} error={e}", user.id);
        return internal_error_response(&l10n, &locale);
    }

    // Dev shortcut: spend the token through the same code path a clicked link takes, so the
    // token and session logic is still exercised locally — only the email is skipped.
    if config.skip_email_verification {
        return match sign_in_with_token(&pool, &config, &raw_token).await {
            Ok(Some((session_cookie, session))) => {
                HttpResponse::Ok()
                    .cookie(session_cookie)
                    .json(RequestLoginResponse {
                        status: "signed_in",
                        session: Some(session),
                    })
            }
            Ok(None) => error_response(
                &l10n,
                &locale,
                StatusCode::BAD_REQUEST,
                "auth-token-invalid",
            ),
            Err(e) => {
                error!(
                    "failed to sign in without email user_id={} error={e}",
                    user.id
                );
                internal_error_response(&l10n, &locale)
            }
        };
    }

    let link = format!(
        "{}/auth/verify?token={raw_token}",
        config.client_base_url.trim_end_matches('/')
    );

    // The email goes out in the language the client is showing, not the server's default.
    let email_locale = l10n.locale_or_default(body.language.as_deref());

    if let Err(e) = email::send_magic_link(
        &config,
        &http_client,
        &l10n,
        &email_locale,
        &email_address,
        &link,
    )
    .await
    {
        error!("failed to send magic link error={e}");
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_GATEWAY,
            "auth-email-send-failed",
        );
    }

    // Nothing about the user is echoed back: the response looks identical whether or not the
    // address belongs to an existing account.
    HttpResponse::Ok().json(RequestLoginResponse {
        status: "email_sent",
        session: None,
    })
}

/// `GET /auth/verify?token=…` — spend the magic-link token and open a session.
async fn verify(
    query: web::Query<VerifyQuery>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
    config: web::Data<AuthConfig>,
) -> impl Responder {
    let locale = l10n.locale();

    match sign_in_with_token(&pool, &config, &query.token).await {
        Ok(Some((session_cookie, session))) => {
            HttpResponse::Ok().cookie(session_cookie).json(session)
        }
        Ok(None) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "auth-token-invalid",
        ),
        Err(e) => {
            error!("failed to verify login token error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /auth/me` — the signed-in user and the households they belong to.
async fn me(req: HttpRequest, pool: web::Data<PgPool>, l10n: web::Data<L10n>) -> impl Responder {
    let locale = l10n.locale();

    let user = match current_user(&req, &pool).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return error_response(
                &l10n,
                &locale,
                StatusCode::UNAUTHORIZED,
                "auth-not-signed-in",
            )
        }
        Err(e) => {
            error!("failed to load session user error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };

    match load_current_user(&pool, user).await {
        Ok(session) => HttpResponse::Ok().json(session),
        Err(e) => {
            error!("failed to list memberships error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `POST /auth/logout` — end the session and clear the cookie.
async fn logout(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
    config: web::Data<AuthConfig>,
) -> impl Responder {
    if let Some(token) = session_token(&req) {
        if let Err(e) = repository::delete_session(&pool, &tokens::hash(&token)).await {
            error!("failed to delete session error={e}");
            return internal_error_response(&l10n, &l10n.locale());
        }
    }

    // Clearing the cookie regardless keeps logout idempotent: calling it without a session, or
    // twice, still leaves the browser signed out.
    HttpResponse::NoContent()
        .cookie(build_removal_cookie(&config))
        .finish()
}

/// `POST /auth/onboarding` — create a household (becoming its `family_manager`) or join an
/// existing one by `join_code` (becoming a `family_member`).
async fn onboarding(
    body: web::Json<OnboardingRequest>,
    req: HttpRequest,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();

    let user = match current_user(&req, &pool).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return error_response(
                &l10n,
                &locale,
                StatusCode::UNAUTHORIZED,
                "auth-not-signed-in",
            )
        }
        Err(e) => {
            error!("failed to load session user error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };

    let join_code = body
        .join_code
        .as_deref()
        .map(str::trim)
        .filter(|code| !code.is_empty());

    let (household_id, role) = match join_code {
        Some(code) => match households_repository::get_by_join_code(&pool, code).await {
            Ok(Some(household)) => (household.id, "family_member"),
            Ok(None) => {
                return error_response(
                    &l10n,
                    &locale,
                    StatusCode::NOT_FOUND,
                    "auth-join-code-not-found",
                )
            }
            Err(e) => {
                error!("failed to look up join code error={e}");
                return internal_error_response(&l10n, &locale);
            }
        },
        None => match households_repository::create(&pool).await {
            Ok(household) => (household.id, "family_manager"),
            Err(e) => {
                error!("failed to create household error={e}");
                return internal_error_response(&l10n, &locale);
            }
        },
    };

    let new_member = NewHouseholdMember {
        household_id,
        user_id: user.id,
        r#type: role.to_string(),
    };

    match household_members_repository::create(&pool, &new_member).await {
        Ok(_) => match load_current_user(&pool, user).await {
            Ok(session) => HttpResponse::Created().json(session),
            Err(e) => {
                error!("failed to list memberships after onboarding error={e}");
                internal_error_response(&l10n, &locale)
            }
        },
        Err(e) if is_unique_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::CONFLICT,
            "auth-already-in-household",
        ),
        Err(e) => {
            error!("failed to connect user to household error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Spends a magic-link token and opens a session for whoever it belongs to.
///
/// `Ok(None)` means the token was unknown, expired, or already used — the one path both `verify`
/// and the dev shortcut share.
async fn sign_in_with_token(
    pool: &PgPool,
    config: &AuthConfig,
    raw_token: &str,
) -> Result<Option<(actix_web::cookie::Cookie<'static>, CurrentUser)>, sqlx::Error> {
    let Some(user_id) = repository::consume_login_token(pool, &tokens::hash(raw_token)).await?
    else {
        return Ok(None);
    };

    // Clicking a real link proves the mailbox is theirs. The dev shortcut proves nothing, so it
    // deliberately leaves `status` alone.
    if !config.skip_email_verification {
        repository::mark_user_verified(pool, user_id).await?;
    }

    let session_token = tokens::generate();
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);
    repository::create_session(pool, user_id, &tokens::hash(&session_token), expires_at).await?;

    let Some(user) = users_repository::get(pool, user_id).await? else {
        return Ok(None);
    };

    let session = load_current_user(pool, user).await?;
    Ok(Some((build_session_cookie(config, session_token), session)))
}

/// Pairs a user with the households they belong to.
async fn load_current_user(pool: &PgPool, user: User) -> Result<CurrentUser, sqlx::Error> {
    let households: Vec<Membership> = repository::list_memberships(pool, user.id).await?;
    Ok(CurrentUser { user, households })
}

/// Registers the auth feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/auth/request-login", web::post().to(request_login))
        .route("/auth/verify", web::get().to(verify))
        .route("/auth/me", web::get().to(me))
        .route("/auth/logout", web::post().to(logout))
        .route("/auth/onboarding", web::post().to(onboarding));
}
