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
use crate::shared::l10n::L10n;

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
        .configure(configure)
}

/// Creates a category through `POST /categories` and returns its id, for tests that only need an
/// existing row to act on.
async fn create_via_api<S, B>(app: &S, name_en: &str, name_fr: &str, kind: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/categories")
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
        .unwrap_or_else(|| panic!("expected created category, got {body}"))
}

/// Fetches all categories through `GET /categories`.
async fn list_via_api<S, B>(app: &S) -> Vec<serde_json::Value>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get().uri("/categories").to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body.as_array().unwrap().clone()
}

// --- POST /categories ---

#[sqlx::test]
async fn create_category_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/categories")
        .set_json(serde_json::json!({
            "name_en": "Test Category",
            "name_fr": "Catégorie test",
            "type": "expense",
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
    assert_eq!(body["type"], "expense");
    assert_eq!(location, format!("/categories/{}", body["id"].as_i64().unwrap()));
}

#[sqlx::test]
async fn create_category_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/categories")
        .set_json(serde_json::json!({ "name_en": "Test Category" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_category_rejects_invalid_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/categories")
        .set_json(serde_json::json!({
            "name_en": "Bogus",
            "name_fr": "Bogus",
            "type": "nonsense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 500);
}

#[sqlx::test]
async fn create_category_rejects_duplicate_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "Test Category", "Catégorie test", "expense").await;

    let req = test::TestRequest::post()
        .uri("/categories")
        .set_json(serde_json::json!({
            "name_en": "Test Category",
            "name_fr": "Catégorie test 2",
            "type": "expense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 500);
}

// --- GET /categories ---

#[sqlx::test]
async fn list_categories_returns_all_rows(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let before = list_via_api(&app).await.len();
    create_via_api(&app, "Test Category", "Catégorie test", "expense").await;
    create_via_api(&app, "Test Income", "Revenu test", "income").await;

    let rows = list_via_api(&app).await;

    assert_eq!(rows.len(), before + 2);
}

#[sqlx::test]
async fn list_categories_includes_seeded_categories(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let rows = list_via_api(&app).await;

    assert!(rows
        .iter()
        .any(|r| r["name_en"] == "Groceries" && r["type"] == "expense"));
    assert!(rows
        .iter()
        .any(|r| r["name_en"] == "Salary" && r["type"] == "income"));
}

#[sqlx::test]
async fn list_categories_orders_by_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let first_id = create_via_api(&app, "Test Category", "Catégorie test", "expense").await;
    let second_id = create_via_api(&app, "Test Income", "Revenu test", "income").await;

    let rows = list_via_api(&app).await;
    let ids: Vec<i64> = rows.iter().map(|r| r["id"].as_i64().unwrap()).collect();
    let first_pos = ids.iter().position(|&id| id == first_id).unwrap();
    let second_pos = ids.iter().position(|&id| id == second_id).unwrap();

    assert!(first_pos < second_pos);
}

// --- GET /categories/{id} ---

#[sqlx::test]
async fn get_category_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "Test Category", "Catégorie test", "expense").await;

    let req = test::TestRequest::get()
        .uri(&format!("/categories/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Category");
    assert_eq!(body["type"], "expense");
}

#[sqlx::test]
async fn get_category_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get()
        .uri("/categories/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

// --- PATCH /categories/{id} ---

#[sqlx::test]
async fn update_category_changes_only_given_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "Test Category", "Catégorie test", "expense").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/categories/{id}"))
        .set_json(serde_json::json!({ "name_en": "Test Category & Household" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Category & Household");
    assert_eq!(body["name_fr"], "Catégorie test");
    assert_eq!(body["type"], "expense");
}

#[sqlx::test]
async fn update_category_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::patch()
        .uri("/categories/999999")
        .set_json(serde_json::json!({ "name_en": "Nope" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn update_category_with_empty_body_leaves_row_unchanged(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "Test Category", "Catégorie test", "expense").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/categories/{id}"))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name_en"], "Test Category");
    assert_eq!(body["name_fr"], "Catégorie test");
    assert_eq!(body["type"], "expense");
}

#[sqlx::test]
async fn update_category_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "Test Category", "Catégorie test", "expense").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/categories/{id}"))
        .set_json(serde_json::json!({ "type": true }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- DELETE /categories/{id} ---

#[sqlx::test]
async fn delete_category_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "Test Category", "Catégorie test", "expense").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/categories/{id}"))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/categories/{id}"))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_category_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::delete()
        .uri("/categories/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_category_twice_returns_not_found_second_time(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "Test Category", "Catégorie test", "expense").await;

    let first_req = test::TestRequest::delete()
        .uri(&format!("/categories/{id}"))
        .to_request();
    let first_resp = test::call_service(&app, first_req).await;
    assert_eq!(first_resp.status(), 204);

    let second_req = test::TestRequest::delete()
        .uri(&format!("/categories/{id}"))
        .to_request();
    let second_resp = test::call_service(&app, second_req).await;
    assert_eq!(second_resp.status(), 404);
}
