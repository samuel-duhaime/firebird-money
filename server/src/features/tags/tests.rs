//! Integration tests for the tags HTTP API.
//!
//! Each `#[sqlx::test]` gets its own throwaway Postgres database (migrated from `migrations/`,
//! dropped afterwards), so these never touch real dev data.

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
        .configure(crate::features::transactions::configure)
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

/// Creates a tag through `POST /tags` and returns its id, for tests that only need an existing
/// row to act on.
async fn create_via_api<S, B>(app: &S, cookie: &str, name: &str, color: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/tags")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": name, "color": color }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created tag, got {body}"))
}

/// Fetches all of the caller's tags through `GET /tags`.
async fn list_via_api<S, B>(app: &S, cookie: &str) -> Vec<serde_json::Value>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/tags")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body.as_array().unwrap().clone()
}

// --- POST /tags ---

#[sqlx::test]
async fn create_tag_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/tags")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Vacation 2026", "color": "#2F80ED" }))
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
    assert_eq!(body["name"], "Vacation 2026");
    assert_eq!(body["color"], "#2F80ED");
    assert!(body["household_id"].is_i64());
    assert_eq!(location, format!("/tags/{}", body["id"].as_i64().unwrap()));
}

#[sqlx::test]
async fn create_tag_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/tags")
        .set_json(serde_json::json!({ "name": "Vacation 2026", "color": "#2F80ED" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn create_tag_requires_a_household(pool: PgPool) {
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
        .uri("/tags")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Vacation 2026", "color": "#2F80ED" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 403);
}

#[sqlx::test]
async fn create_tag_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/tags")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Vacation 2026" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_tag_rejects_duplicate_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;

    let req = test::TestRequest::post()
        .uri("/tags")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Vacation 2026", "color": "#27AE60" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

// --- GET /tags ---

#[sqlx::test]
async fn list_tags_returns_all_rows(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let before = list_via_api(&app, &cookie).await.len();
    create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;
    create_via_api(&app, &cookie, "Reimbursable", "#27AE60").await;

    let rows = list_via_api(&app, &cookie).await;

    assert_eq!(rows.len(), before + 2);
}

#[sqlx::test]
async fn list_tags_includes_seeded_tags(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let rows = list_via_api(&app, &cookie).await;

    assert!(rows.iter().any(|r| r["name"] == "Tax"));
    assert!(rows.iter().any(|r| r["name"] == "Reimburse"));
    assert!(rows.iter().any(|r| r["name"] == "Member #1"));
}

#[sqlx::test]
async fn list_tags_orders_by_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let first_id = create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;
    let second_id = create_via_api(&app, &cookie, "Reimbursable", "#27AE60").await;

    let rows = list_via_api(&app, &cookie).await;
    let ids: Vec<i64> = rows.iter().map(|r| r["id"].as_i64().unwrap()).collect();
    let first_pos = ids.iter().position(|&id| id == first_id).unwrap();
    let second_pos = ids.iter().position(|&id| id == second_id).unwrap();

    assert!(first_pos < second_pos);
}

#[sqlx::test]
async fn list_tags_does_not_leak_another_households_tags(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let only_as_id = create_via_api(&app, &cookie_a, "Only A's Tag", "#2F80ED").await;

    let rows = list_via_api(&app, &cookie_b).await;

    assert!(!rows.iter().any(|r| r["id"].as_i64() == Some(only_as_id)));
}

// --- GET /tags/{id} ---

#[sqlx::test]
async fn get_tag_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;

    let req = test::TestRequest::get()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Vacation 2026");
}

#[sqlx::test]
async fn get_tag_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/tags/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn get_tag_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let id = create_via_api(&app, &cookie_a, "Only A's Tag", "#2F80ED").await;

    let req = test::TestRequest::get()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /tags/{id} ---

#[sqlx::test]
async fn update_tag_changes_only_given_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Vacation 2027" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Vacation 2027");
    assert_eq!(body["color"], "#2F80ED");
}

#[sqlx::test]
async fn update_tag_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::patch()
        .uri("/tags/999999")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Nope" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn update_tag_with_empty_body_leaves_row_unchanged(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Vacation 2026");
    assert_eq!(body["color"], "#2F80ED");
}

#[sqlx::test]
async fn update_tag_rejects_duplicate_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;
    let id = create_via_api(&app, &cookie, "Reimbursable", "#27AE60").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Vacation 2026" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

#[sqlx::test]
async fn update_tag_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let id = create_via_api(&app, &cookie_a, "Only A's Tag", "#2F80ED").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "name": "Hijacked" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /tags/{id} ---

#[sqlx::test]
async fn delete_tag_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_tag_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/tags/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_tag_twice_returns_not_found_second_time(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;

    let first_req = test::TestRequest::delete()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let first_resp = test::call_service(&app, first_req).await;
    assert_eq!(first_resp.status(), 204);

    let second_req = test::TestRequest::delete()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let second_resp = test::call_service(&app, second_req).await;
    assert_eq!(second_resp.status(), 404);
}

#[sqlx::test]
async fn delete_tag_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let id = create_via_api(&app, &cookie_a, "Only A's Tag", "#2F80ED").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/tags/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 404);
}

#[sqlx::test]
async fn delete_tag_rejects_when_referenced_by_transaction(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let tag_id = create_via_api(&app, &cookie, "Vacation 2026", "#2F80ED").await;

    let category_id = any_seeded_category_id(&pool).await;

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
    let txn_body: serde_json::Value = test::read_body_json(txn_resp).await;
    let transaction_id = txn_body["id"].as_i64().unwrap();

    sqlx::query("INSERT INTO transaction_tags (transaction_id, tag_id) VALUES ($1, $2)")
        .bind(transaction_id)
        .bind(tag_id as i32)
        .execute(&pool)
        .await
        .unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/tags/{tag_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 409);
    let body: serde_json::Value = test::read_body_json(delete_resp).await;
    assert!(body["error"].is_string());
}

/// Fetches the id of any one category that already exists in the database (seeded by onboarding),
/// so `delete_tag_rejects_when_referenced_by_transaction` can create a transaction without
/// depending on the categories feature being wired into this test app.
async fn any_seeded_category_id(pool: &PgPool) -> i32 {
    sqlx::query_scalar::<_, i32>("SELECT id FROM categories ORDER BY id LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("expected at least one seeded category")
}
