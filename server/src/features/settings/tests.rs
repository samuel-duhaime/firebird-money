//! Integration tests for the settings HTTP API. `GET`/`PATCH /settings` always act on the
//! signed-in caller's own row — there's no id in the URL.
//!
//! Each `#[sqlx::test]` gets its own throwaway Postgres database (migrated from
//! `migrations/`, dropped afterwards), so these never touch real dev data.

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App};
use sqlx::PgPool;

use super::handlers::configure;
use crate::features::auth;
use crate::shared::config::{AuthConfig, SESSION_COOKIE_NAME};
use crate::shared::l10n::L10n;

/// An `AuthConfig` for tests: no mail provider, no production flags.
fn test_config() -> AuthConfig {
    AuthConfig {
        app_env: "development".to_string(),
        skip_email_verification: true,
        resend_api_key: None,
        email_from: "FireBird Money <test@example.com>".to_string(),
        client_base_url: "http://localhost:5173".to_string(),
    }
}

fn app_with(
    pool: PgPool,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(pool))
        .app_data(web::Data::new(L10n::new()))
        .app_data(web::Data::new(test_config()))
        .app_data(web::Data::new(reqwest::Client::new()))
        .configure(configure)
        .configure(auth::configure)
}

/// Signs in through `POST /auth/request-login` and returns the session cookie to replay on later
/// requests (the test client doesn't keep a cookie jar).
async fn sign_in<S, B>(app: &S, email: &str) -> String
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/auth/request-login")
        .set_json(serde_json::json!({ "email": email }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200);

    let cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .unwrap_or_else(|| panic!("expected a {SESSION_COOKIE_NAME} cookie"));

    format!("{}={}", cookie.name(), cookie.value())
}

// --- GET /settings ---

#[sqlx::test]
async fn get_settings_defaults_to_every_column_visible(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/settings")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["show_category_column"], true);
    assert_eq!(body["show_tags_column"], true);
    assert_eq!(body["show_account_column"], true);
}

#[sqlx::test]
async fn get_settings_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get().uri("/settings").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

// --- PATCH /settings ---

#[sqlx::test]
async fn update_settings_persists_a_partial_change(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let patch_req = test::TestRequest::patch()
        .uri("/settings")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({ "show_category_column": false }))
        .to_request();
    let patch_resp = test::call_service(&app, patch_req).await;

    assert_eq!(patch_resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(patch_resp).await;
    assert_eq!(body["show_category_column"], false);
    // Untouched fields keep their default.
    assert_eq!(body["show_tags_column"], true);
    assert_eq!(body["show_account_column"], true);

    let get_req = test::TestRequest::get()
        .uri("/settings")
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_body: serde_json::Value = test::call_and_read_body_json(&app, get_req).await;
    assert_eq!(get_body["show_category_column"], false);
}

#[sqlx::test]
async fn update_settings_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::patch()
        .uri("/settings")
        .set_json(serde_json::json!({ "show_category_column": false }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn update_settings_only_affects_the_caller(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let sam_cookie = sign_in(&app, "sam@example.com").await;
    let jane_cookie = sign_in(&app, "jane@example.com").await;

    let patch_req = test::TestRequest::patch()
        .uri("/settings")
        .insert_header(("Cookie", sam_cookie))
        .set_json(serde_json::json!({ "show_account_column": false }))
        .to_request();
    assert_eq!(test::call_service(&app, patch_req).await.status(), 200);

    let get_req = test::TestRequest::get()
        .uri("/settings")
        .insert_header(("Cookie", jane_cookie))
        .to_request();
    let jane_settings: serde_json::Value = test::call_and_read_body_json(&app, get_req).await;
    assert_eq!(jane_settings["show_account_column"], true);
}
