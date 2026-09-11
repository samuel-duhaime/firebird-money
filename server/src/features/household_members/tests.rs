//! Integration tests for the household members HTTP API. Every route is scoped to the caller's
//! own household — see `handlers.rs`.
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
        .configure(crate::features::households::configure)
        .configure(crate::features::users::configure)
}

/// Signs in and completes onboarding (creating a fresh household, making the caller its
/// `family_manager`), returning the session cookie.
async fn sign_in_with_household<S, B>(app: &S, email: &str) -> String
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
    let cookie = format!("{}={}", cookie.name(), cookie.value());

    let onboard_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({}))
        .to_request();
    assert_eq!(test::call_service(app, onboard_req).await.status(), 201);

    cookie
}

async fn create_user<S, B>(app: &S, cookie: &str, email: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/users")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "email": email }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"].as_i64().unwrap()
}

/// Connects `user_id` to `cookie`'s own household via `POST /household-members` — the caller must
/// already be that household's `family_manager`.
async fn create_member_via_api<S, B>(app: &S, cookie: &str, user_id: i64, member_type: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "user_id": user_id,
            "type": member_type,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created household member, got {body}"))
}

// --- POST /household-members ---

#[sqlx::test]
async fn create_household_member_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let user_id = create_user(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "user_id": user_id,
            "type": "family_member",
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
    assert_eq!(body["user_id"].as_i64().unwrap(), user_id);
    assert_eq!(body["type"], "family_member");
    assert!(body["household_id"].is_i64());
    assert_eq!(
        location,
        format!("/household-members/{}", body["id"].as_i64().unwrap())
    );
}

#[sqlx::test]
async fn create_household_member_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/household-members")
        .set_json(serde_json::json!({
            "user_id": 1,
            "type": "family_manager",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn create_household_member_requires_a_family_manager(pool: PgPool) {
    // The caller onboards by *joining* an existing household, becoming a family_member — not
    // its manager — so this must be rejected even though they do belong to the household.
    let app = test::init_service(app_with(pool)).await;
    let manager_cookie = sign_in_with_household(&app, "manager@example.com").await;

    let me_req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", manager_cookie.clone()))
        .to_request();
    let me: serde_json::Value = test::call_and_read_body_json(&app, me_req).await;
    let join_code = me["household"]["join_code"].as_str().unwrap();

    let member_cookie =
        sign_in_with_household_via_join(&app, "member@example.com", join_code).await;
    let user_id = create_user(&app, &member_cookie, "jane@example.com").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", member_cookie))
        .set_json(serde_json::json!({
            "user_id": user_id,
            "type": "family_member",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 403);
}

/// Like `sign_in_with_household`, but joins an existing household by `join_code` instead of
/// creating a new one — the caller becomes a `family_member`, not a manager.
async fn sign_in_with_household_via_join<S, B>(app: &S, email: &str, join_code: &str) -> String
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
        .unwrap();
    let cookie = format!("{}={}", cookie.name(), cookie.value());

    let onboard_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({ "join_code": join_code }))
        .to_request();
    assert_eq!(test::call_service(app, onboard_req).await.status(), 201);

    cookie
}

#[sqlx::test]
async fn create_household_member_rejects_invalid_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let user_id = create_user(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "user_id": user_id,
            "type": "nonsense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_household_member_rejects_unknown_user(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "user_id": 999999,
            "type": "family_manager",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_household_member_rejects_duplicate_pair(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let user_id = create_user(&app, &cookie, "jane@example.com").await;
    create_member_via_api(&app, &cookie, user_id, "family_member").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "user_id": user_id,
            "type": "family_member",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

#[sqlx::test]
async fn create_household_member_rejects_a_second_household_for_the_same_user(pool: PgPool) {
    // A user belongs to exactly one household ever. Jane is already a member of A's household;
    // B (manager of a different household) must not be able to add her to theirs too.
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let jane_id = create_user(&app, &cookie_a, "jane@example.com").await;
    create_member_via_api(&app, &cookie_a, jane_id, "family_member").await;

    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({
            "user_id": jane_id,
            "type": "family_member",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

// --- GET /household-members ---

#[sqlx::test]
async fn list_household_members_returns_only_the_callers_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let jane_id = create_user(&app, &cookie_a, "jane@example.com").await;
    create_member_via_api(&app, &cookie_a, jane_id, "family_member").await;

    let cookie_b = sign_in_with_household(&app, "b@example.com").await;

    let req = test::TestRequest::get()
        .uri("/household-members")
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();

    // Only B themselves — not the manager or Jane from A's household.
    assert_eq!(rows.len(), 1);
    assert!(!rows.iter().any(|r| r["user_id"].as_i64() == Some(jane_id)));
}

#[sqlx::test]
async fn list_household_members_filters_by_user(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let jane_id = create_user(&app, &cookie, "jane@example.com").await;
    create_member_via_api(&app, &cookie, jane_id, "family_member").await;

    let req = test::TestRequest::get()
        .uri(&format!("/household-members?user_id={jane_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["user_id"].as_i64().unwrap(), jane_id);
}

// --- GET /household-members/{id} ---

#[sqlx::test]
async fn get_household_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/household-members/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn get_household_member_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let jane_id = create_user(&app, &cookie_a, "jane@example.com").await;
    let id = create_member_via_api(&app, &cookie_a, jane_id, "family_member").await;

    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let req = test::TestRequest::get()
        .uri(&format!("/household-members/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /household-members/{id} ---

#[sqlx::test]
async fn update_household_member_changes_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let user_id = create_user(&app, &cookie, "jane@example.com").await;
    let id = create_member_via_api(&app, &cookie, user_id, "family_member").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "type": "family_manager" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["type"], "family_manager");
}

#[sqlx::test]
async fn update_household_member_rejects_invalid_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let user_id = create_user(&app, &cookie, "jane@example.com").await;
    let id = create_member_via_api(&app, &cookie, user_id, "family_member").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "type": "nonsense" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn update_household_member_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let jane_id = create_user(&app, &cookie_a, "jane@example.com").await;
    let id = create_member_via_api(&app, &cookie_a, jane_id, "family_member").await;

    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "type": "family_manager" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /household-members/{id} ---

#[sqlx::test]
async fn delete_household_member_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let user_id = create_user(&app, &cookie, "jane@example.com").await;
    let id = create_member_via_api(&app, &cookie, user_id, "family_member").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/household-members/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/household-members/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_household_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/household-members/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_household_member_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::delete()
        .uri("/household-members/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn delete_household_member_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let jane_id = create_user(&app, &cookie_a, "jane@example.com").await;
    let id = create_member_via_api(&app, &cookie_a, jane_id, "family_member").await;

    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/household-members/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 404);
}
