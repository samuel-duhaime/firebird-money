//! Integration tests for the category groups HTTP API.
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

async fn create_via_api<S, B>(
    app: &S,
    cookie: &str,
    name_en: &str,
    name_fr: &str,
    kind: &str,
) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "name_en": name_en,
            "name_fr": name_fr,
            "type": kind,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created category group, got {body}"))
}

// --- POST /category-groups ---

#[sqlx::test]
async fn create_category_group_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "name_en": "Test Group",
            "name_fr": "Groupe test",
            "type": "expense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Group");
    assert_eq!(body["type"], "expense");
    assert!(body["household_id"].is_i64());
}

#[sqlx::test]
async fn create_category_group_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/category-groups")
        .set_json(serde_json::json!({
            "name_en": "Test Group",
            "name_fr": "Groupe test",
            "type": "expense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn create_category_group_requires_a_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/auth/request-login")
        .set_json(serde_json::json!({ "email": "sam@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .unwrap();
    let cookie = format!("{}={}", cookie.name(), cookie.value());

    let req = test::TestRequest::post()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "name_en": "Test Group",
            "name_fr": "Groupe test",
            "type": "expense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 403);
}

#[sqlx::test]
async fn create_category_group_rejects_invalid_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "name_en": "Bogus",
            "name_fr": "Bogus",
            "type": "nonsense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_category_group_rejects_duplicate_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    create_via_api(&app, &cookie, "Test Group", "Groupe test", "expense").await;

    let req = test::TestRequest::post()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "name_en": "Test Group",
            "name_fr": "Groupe test 2",
            "type": "expense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

// --- GET /category-groups ---

#[sqlx::test]
async fn list_category_groups_includes_seeded_groups(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    assert_eq!(
        rows.len(),
        16,
        "every household starts with the same starter groups"
    );
    assert!(rows
        .iter()
        .any(|r| r["name_en"] == "Food & Dining" && r["type"] == "expense"));
    assert!(rows
        .iter()
        .any(|r| r["name_en"] == "Income" && r["type"] == "income"));
    assert!(rows
        .iter()
        .any(|r| r["name_en"] == "Savings & Investments" && r["type"] == "transfer"));
}

#[sqlx::test]
async fn list_category_groups_does_not_leak_another_households_groups(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let custom_id =
        create_via_api(&app, &cookie_a, "Only A's Group", "Groupe de A", "expense").await;

    let req = test::TestRequest::get()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    assert!(!rows.iter().any(|r| r["id"].as_i64() == Some(custom_id)));
}

// --- GET /category-groups/{id} ---

#[sqlx::test]
async fn get_category_group_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Test Group", "Groupe test", "expense").await;

    let req = test::TestRequest::get()
        .uri(&format!("/category-groups/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Group");
}

#[sqlx::test]
async fn get_category_group_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/category-groups/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn get_category_group_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let id = create_via_api(&app, &cookie_a, "Only A's Group", "Groupe de A", "expense").await;

    let req = test::TestRequest::get()
        .uri(&format!("/category-groups/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /category-groups/{id} ---

#[sqlx::test]
async fn update_category_group_changes_only_given_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Test Group", "Groupe test", "expense").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/category-groups/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name_en": "Renamed Group" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Renamed Group");
    assert_eq!(body["name_fr"], "Groupe test");
    assert_eq!(body["type"], "expense");
}

#[sqlx::test]
async fn update_category_group_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let id = create_via_api(&app, &cookie_a, "Only A's Group", "Groupe de A", "expense").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/category-groups/{id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "name_en": "Hijacked" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /category-groups/{id} ---

#[sqlx::test]
async fn delete_category_group_rejects_when_it_has_categories(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let list_req = test::TestRequest::get()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let groups: serde_json::Value = test::call_and_read_body_json(&app, list_req).await;
    let seeded_group_id = groups[0]["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/category-groups/{seeded_group_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 409);
}

#[sqlx::test]
async fn delete_category_group_removes_an_empty_group(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Test Group", "Groupe test", "expense").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/category-groups/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/category-groups/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_category_group_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/category-groups/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}
