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
        .configure(crate::features::categories::configure)
        .configure(crate::features::institutions::configure)
        .configure(crate::features::accounts::configure)
        .configure(crate::features::transactions::configure)
}

/// Signs in as `email` and completes onboarding, returning the session cookie: with a
/// `join_code` they join that household as a `family_member`; without one they create a new
/// household and become its `family_manager`.
async fn onboard<S, B>(app: &S, email: &str, join_code: Option<&str>) -> String
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
        .set_json(serde_json::json!({
            "first_name": "Test",
            "last_name": "User",
            "join_code": join_code,
        }))
        .to_request();
    assert_eq!(test::call_service(app, onboard_req).await.status(), 201);

    cookie
}

/// `GET /auth/me` for `cookie`.
async fn me<S, B>(app: &S, cookie: &str) -> serde_json::Value
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    test::call_and_read_body_json(app, req).await
}

/// Has `email` join `manager_cookie`'s household by its code, returning the new member's cookie and
/// user id.
async fn join<S, B>(app: &S, manager_cookie: &str, email: &str) -> (String, i64)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let join_code = me(app, manager_cookie).await["household"]["join_code"]
        .as_str()
        .unwrap()
        .to_string();
    let cookie = onboard(app, email, Some(&join_code)).await;
    let id = me(app, &cookie).await["user"]["id"].as_i64().unwrap();
    (cookie, id)
}

// --- GET /household-members ---

#[sqlx::test]
async fn list_household_members_returns_only_the_callers_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;

    let req = test::TestRequest::get()
        .uri("/household-members")
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    // Only B themselves — not the manager or Jane from A's household.
    assert_eq!(rows.len(), 1);
    assert!(!rows.iter().any(|r| r["id"].as_i64() == Some(jane_id)));
}

#[sqlx::test]
async fn list_household_members_includes_everyone_who_joined(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::get()
        .uri("/household-members")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["id"].as_i64().unwrap(), jane_id);
    assert_eq!(rows[1]["role"], "family_member");
}

#[sqlx::test]
async fn list_household_members_includes_each_members_user_details(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;

    let req = test::TestRequest::get()
        .uri("/household-members")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["email"], "sam@example.com");
    assert_eq!(rows[0]["first_name"], "Test");
    assert_eq!(rows[0]["last_name"], "User");
    assert_eq!(rows[0]["status"], "pending");
    assert_eq!(rows[0]["role"], "family_manager");
}

#[sqlx::test]
async fn list_household_members_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get()
        .uri("/household-members")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

// --- GET /household-members/{id} ---

#[sqlx::test]
async fn get_household_member_returns_the_member(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["id"].as_i64().unwrap(), jane_id);
    assert_eq!(body["email"], "jane@example.com");
    assert_eq!(body["role"], "family_member");
}

#[sqlx::test]
async fn get_household_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;

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
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;
    let req = test::TestRequest::get()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /household-members/{id} ---

#[sqlx::test]
async fn update_household_member_changes_role(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (jane_cookie, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "role": "family_manager" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["role"], "family_manager");
    assert_eq!(
        me(&app, &jane_cookie).await["household"]["role"],
        "family_manager"
    );
}

#[sqlx::test]
async fn update_household_member_rejects_invalid_role(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "role": "nonsense" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn update_household_member_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;
    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "role": "family_manager" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /household-members/{id} ---

#[sqlx::test]
async fn delete_household_member_removes_them_from_the_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (jane_cookie, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    assert_eq!(test::call_service(&app, delete_req).await.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    assert_eq!(test::call_service(&app, get_req).await.status(), 404);

    // Her account is kept, just without a household.
    let jane = me(&app, &jane_cookie).await;
    assert_eq!(jane["user"]["id"].as_i64().unwrap(), jane_id);
    assert!(jane["household"].is_null());
}

#[sqlx::test]
async fn delete_household_member_is_blocked_while_they_have_transactions(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (jane_cookie, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let categories_req = test::TestRequest::get()
        .uri("/categories")
        .insert_header(("Cookie", jane_cookie.clone()))
        .to_request();
    let categories: serde_json::Value = test::call_and_read_body_json(&app, categories_req).await;
    let category_id = categories[0]["id"].as_i64().unwrap();
    let account_id = crate::shared::test_support::test_account_id(&app, &jane_cookie).await;

    let transaction_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", jane_cookie))
        .set_json(serde_json::json!({
            "date": "2026-10-15",
            "original_statement": "STARBUCKS",
            "amount": "4.50",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, transaction_req).await.status(),
        201
    );

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;

    assert_eq!(resp.status(), 409);
}

#[sqlx::test]
async fn delete_household_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;

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
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/household-members/{jane_id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;

    assert_eq!(resp.status(), 404);
}
