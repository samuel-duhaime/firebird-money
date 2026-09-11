//! Integration tests for the users HTTP API. `GET`/`PATCH`/`DELETE /users/{id}` are restricted to
//! the signed-in caller's own record — see `handlers.rs`.
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

/// Signs in and returns both the session cookie and the caller's own `user.id` (per
/// `GET /auth/me`) — `GET`/`PATCH`/`DELETE /users/{id}` only ever work on this id.
async fn sign_in_with_id<S, B>(app: &S, email: &str) -> (String, i64)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let cookie = sign_in(app, email).await;

    let me_req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let me: serde_json::Value = test::call_and_read_body_json(app, me_req).await;
    let user_id = me["user"]["id"].as_i64().unwrap();

    (cookie, user_id)
}

// --- POST /users ---

#[sqlx::test]
async fn create_user_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/users")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "email": "jane@example.com",
            "google_id": "google-123",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let location = resp
        .headers()
        .get("Location")
        .expect("Location header")
        .to_str()
        .unwrap()
        .to_string();

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "jane@example.com");
    assert!(!body.as_object().unwrap().contains_key("google_id"));
    assert_eq!(body["status"], "pending");
    assert!(body["first_name"].is_null());
    assert_eq!(location, format!("/users/{}", body["id"].as_i64().unwrap()));
}

#[sqlx::test]
async fn create_user_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/users")
        .set_json(serde_json::json!({ "email": "jane@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn create_user_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/users")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_user_rejects_duplicate_email(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/users")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "email": "sam@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

// --- GET /users/{id} ---

#[sqlx::test]
async fn get_user_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (cookie, id) = sign_in_with_id(&app, "jane@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!("/users/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["email"], "jane@example.com");
}

#[sqlx::test]
async fn get_user_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/users/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn get_user_from_another_user_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (_cookie_a, id_a) = sign_in_with_id(&app, "a@example.com").await;
    let (cookie_b, _id_b) = sign_in_with_id(&app, "b@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!("/users/{id_a}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /users/{id} ---

#[sqlx::test]
async fn update_user_changes_only_given_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (cookie, id) = sign_in_with_id(&app, "jane@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/users/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "first_name": "Jane", "status": "verified" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["first_name"], "Jane");
    assert_eq!(body["status"], "verified");
    assert_eq!(body["email"], "jane@example.com");
}

#[sqlx::test]
async fn update_user_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::patch()
        .uri("/users/999999")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "first_name": "Nope" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn update_user_rejects_invalid_status(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (cookie, id) = sign_in_with_id(&app, "jane@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/users/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "status": "nonsense" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn update_user_from_another_user_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (_cookie_a, id_a) = sign_in_with_id(&app, "a@example.com").await;
    let (cookie_b, _id_b) = sign_in_with_id(&app, "b@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/users/{id_a}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "first_name": "Hijacked" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /users/{id} ---

#[sqlx::test]
async fn delete_user_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (cookie, id) = sign_in_with_id(&app, "jane@example.com").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/users/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/users/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    // 401, not 404: deleting the account also invalidated its own session.
    assert_eq!(get_resp.status(), 401);
}

#[sqlx::test]
async fn delete_user_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/users/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_user_from_another_user_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (_cookie_a, id_a) = sign_in_with_id(&app, "a@example.com").await;
    let (cookie_b, _id_b) = sign_in_with_id(&app, "b@example.com").await;

    let req = test::TestRequest::delete()
        .uri(&format!("/users/{id_a}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_user_rejects_when_referenced_by_member(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (cookie, id) = sign_in_with_id(&app, "jane@example.com").await;

    let onboard_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({}))
        .to_request();
    assert_eq!(test::call_service(&app, onboard_req).await.status(), 201);

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/users/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 409);
    let body: serde_json::Value = test::read_body_json(delete_resp).await;
    assert!(body["error"].is_string());
}
