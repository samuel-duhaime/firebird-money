//! Integration tests for the categories HTTP API.
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
        .configure(crate::features::category_groups::configure)
}

/// Signs in and completes onboarding (creating a fresh household), returning the session cookie.
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

/// Creates a category group through `POST /category-groups` and returns its id, so tests have a
/// `group_id` to create custom categories against without depending on the seeded defaults.
async fn create_group_via_api<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "name_en": "Test Group",
            "name_fr": "Groupe test",
            "type": "expense",
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"].as_i64().unwrap()
}

/// Creates a category through `POST /categories` and returns its id, for tests that only need an
/// existing row to act on.
async fn create_via_api<S, B>(
    app: &S,
    cookie: &str,
    group_id: i64,
    name_en: &str,
    name_fr: &str,
) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "group_id": group_id,
            "name_en": name_en,
            "name_fr": name_fr,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created category, got {body}"))
}

/// Fetches all of the caller's categories through `GET /categories`.
async fn list_via_api<S, B>(app: &S, cookie: &str) -> Vec<serde_json::Value>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body.as_array().unwrap().clone()
}

// --- POST /categories ---

#[sqlx::test]
async fn create_category_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;

    let req = test::TestRequest::post()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "group_id": group_id,
            "name_en": "Test Category",
            "name_fr": "Catégorie test",
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
    assert_eq!(body["name_en"], "Test Category");
    assert_eq!(body["name_fr"], "Catégorie test");
    assert_eq!(body["group_id"].as_i64().unwrap(), group_id);
    assert!(body["household_id"].is_i64());
    assert_eq!(
        location,
        format!("/categories/{}", body["id"].as_i64().unwrap())
    );
}

#[sqlx::test]
async fn create_category_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/categories")
        .set_json(serde_json::json!({
            "group_id": 1,
            "name_en": "Test Category",
            "name_fr": "Catégorie test",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn create_category_requires_a_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let login_req = test::TestRequest::post()
        .uri("/auth/request-login")
        .set_json(serde_json::json!({ "email": "sam@example.com" }))
        .to_request();
    let login_resp = test::call_service(&app, login_req).await;
    let cookie = login_resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .unwrap();
    let cookie = format!("{}={}", cookie.name(), cookie.value());

    let req = test::TestRequest::post()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "group_id": 1,
            "name_en": "Test Category",
            "name_fr": "Catégorie test",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 403);
}

#[sqlx::test]
async fn create_category_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name_en": "Test Category" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_category_rejects_unknown_group(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "group_id": 999999,
            "name_en": "Bogus",
            "name_fr": "Bogus",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn create_category_rejects_duplicate_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let req = test::TestRequest::post()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "group_id": group_id,
            "name_en": "Test Category",
            "name_fr": "Catégorie test 2",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

// --- GET /categories ---

#[sqlx::test]
async fn list_categories_returns_all_rows(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let before = list_via_api(&app, &cookie).await.len();
    create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;
    create_via_api(&app, &cookie, group_id, "Test Income", "Revenu test").await;

    let rows = list_via_api(&app, &cookie).await;

    assert_eq!(rows.len(), before + 2);
}

#[sqlx::test]
async fn list_categories_includes_seeded_categories(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let rows = list_via_api(&app, &cookie).await;

    assert!(rows.iter().any(|r| r["name_en"] == "Groceries"));
    assert!(rows.iter().any(|r| r["name_en"] == "Paychecks"));
}

#[sqlx::test]
async fn list_categories_orders_by_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let first_id = create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;
    let second_id = create_via_api(&app, &cookie, group_id, "Test Income", "Revenu test").await;

    let rows = list_via_api(&app, &cookie).await;
    let ids: Vec<i64> = rows.iter().map(|r| r["id"].as_i64().unwrap()).collect();
    let first_pos = ids.iter().position(|&id| id == first_id).unwrap();
    let second_pos = ids.iter().position(|&id| id == second_id).unwrap();

    assert!(first_pos < second_pos);
}

#[sqlx::test]
async fn list_categories_does_not_leak_another_households_categories(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let group_a = create_group_via_api(&app, &cookie_a).await;
    let only_as_id = create_via_api(
        &app,
        &cookie_a,
        group_a,
        "Only A's Category",
        "Catégorie de A",
    )
    .await;

    let rows = list_via_api(&app, &cookie_b).await;

    assert!(!rows.iter().any(|r| r["id"].as_i64() == Some(only_as_id)));
}

// --- GET /categories/{id} ---

#[sqlx::test]
async fn get_category_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let id = create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let req = test::TestRequest::get()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Category");
}

#[sqlx::test]
async fn get_category_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/categories/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn get_category_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let group_a = create_group_via_api(&app, &cookie_a).await;
    let id = create_via_api(
        &app,
        &cookie_a,
        group_a,
        "Only A's Category",
        "Catégorie de A",
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /categories/{id} ---

#[sqlx::test]
async fn update_category_changes_only_given_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let id = create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name_en": "Test Category & Household" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Category & Household");
    assert_eq!(body["name_fr"], "Catégorie test");
    assert_eq!(body["group_id"].as_i64().unwrap(), group_id);
}

#[sqlx::test]
async fn update_category_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::patch()
        .uri("/categories/999999")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name_en": "Nope" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn update_category_with_empty_body_leaves_row_unchanged(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let id = create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Category");
    assert_eq!(body["name_fr"], "Catégorie test");
}

#[sqlx::test]
async fn update_category_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let id = create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "group_id": "not-a-number" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn update_category_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let group_a = create_group_via_api(&app, &cookie_a).await;
    let id = create_via_api(
        &app,
        &cookie_a,
        group_a,
        "Only A's Category",
        "Catégorie de A",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "name_en": "Hijacked" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /categories/{id} ---

#[sqlx::test]
async fn delete_category_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let id = create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_category_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/categories/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_category_twice_returns_not_found_second_time(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let id = create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let first_req = test::TestRequest::delete()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let first_resp = test::call_service(&app, first_req).await;
    assert_eq!(first_resp.status(), 204);

    let second_req = test::TestRequest::delete()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let second_resp = test::call_service(&app, second_req).await;
    assert_eq!(second_resp.status(), 404);
}

#[sqlx::test]
async fn delete_category_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let group_a = create_group_via_api(&app, &cookie_a).await;
    let id = create_via_api(
        &app,
        &cookie_a,
        group_a,
        "Only A's Category",
        "Catégorie de A",
    )
    .await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/categories/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 404);
}

#[sqlx::test]
async fn delete_category_rejects_when_referenced_by_transaction(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(web::Data::new(L10n::new()))
            .app_data(web::Data::new(test_config()))
            .app_data(web::Data::new(reqwest::Client::new()))
            .configure(configure)
            .configure(auth::configure)
            .configure(crate::features::category_groups::configure)
            .configure(crate::features::transactions::configure),
    )
    .await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let group_id = create_group_via_api(&app, &cookie).await;
    let category_id =
        create_via_api(&app, &cookie, group_id, "Test Category", "Catégorie test").await;

    let txn_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account": "User 1",
        }))
        .to_request();
    let txn_resp = test::call_service(&app, txn_req).await;
    assert_eq!(txn_resp.status(), 201);

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/categories/{category_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 409);
    let body: serde_json::Value = test::read_body_json(delete_resp).await;
    assert!(body["error"].is_string());
}
