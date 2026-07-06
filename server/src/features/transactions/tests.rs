//! Integration tests for the transactions HTTP API.
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

/// Creates a transaction through `POST /transactions` and returns its id, for tests that only
/// need an existing row to act on.
async fn create_via_api<S, B>(app: &S, date: &str, merchant: &str, amount: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": date,
            "merchant": merchant,
            "amount": amount,
            "category_id": 1,
            "account": "User 1",
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"].as_i64().unwrap()
}

// --- GET /transactions ---

#[sqlx::test]
async fn list_transactions_returns_all_rows(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get().uri("/transactions").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[sqlx::test]
async fn list_transactions_filters_by_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA SUPERMARKT", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?merchant=iga")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "IGA SUPERMARKT");
}

#[sqlx::test]
async fn list_transactions_filters_by_date(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?date=2024-01-15")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_date_and_merchant_combined(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-15", "IGA", "56.78").await;
    create_via_api(&app, "2024-01-16", "IGA", "78.90").await;

    let req = test::TestRequest::get()
        .uri("/transactions?date=2024-01-15&merchant=iga")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "IGA");
    assert_eq!(rows[0]["date"], "2024-01-15");
}

#[sqlx::test]
async fn list_transactions_returns_empty_array_when_no_matches(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::get()
        .uri("/transactions?merchant=nonexistent")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn list_transactions_orders_by_date_desc(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get().uri("/transactions").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    let dates: Vec<&str> = rows.iter().map(|r| r["date"].as_str().unwrap()).collect();
    assert_eq!(dates, vec!["2024-01-17", "2024-01-16", "2024-01-15"]);
}

// --- GET /transactions/{id} ---

#[sqlx::test]
async fn get_transaction_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["merchant"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
}

#[sqlx::test]
async fn get_transaction_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get()
        .uri("/transactions/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

// --- POST /transactions ---

#[sqlx::test]
async fn create_transaction_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": 1,
            "account": "User 1",
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
    assert_eq!(body["merchant"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(location, format!("/transactions/{}", body["id"].as_i64().unwrap()));
}

#[sqlx::test]
async fn create_transaction_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({ "merchant": "STARBUCKS" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_transaction_persists_all_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": 7,
            "account": "User 2",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["date"], "2024-01-15");
    assert_eq!(body["merchant"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["category_id"], 7);
    assert_eq!(body["account"], "User 2");
}

#[sqlx::test]
async fn create_transaction_rejects_invalid_date(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": "not-a-date",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": 1,
            "account": "User 1",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- PATCH /transactions/{id} ---

#[sqlx::test]
async fn update_transaction_changes_only_given_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .set_json(serde_json::json!({ "amount": "20.00" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "20.00");
    assert_eq!(body["merchant"], "STARBUCKS");
}

#[sqlx::test]
async fn update_transaction_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::patch()
        .uri("/transactions/999999")
        .set_json(serde_json::json!({ "amount": "20.00" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn update_transaction_with_empty_body_leaves_row_unchanged(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["merchant"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["date"], "2024-01-15");
}

#[sqlx::test]
async fn update_transaction_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .set_json(serde_json::json!({ "amount": true }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- DELETE /transactions/{id} ---

#[sqlx::test]
async fn delete_transaction_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/transactions/{id}"))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_transaction_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::delete()
        .uri("/transactions/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_transaction_twice_returns_not_found_second_time(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let first_req = test::TestRequest::delete()
        .uri(&format!("/transactions/{id}"))
        .to_request();
    let first_resp = test::call_service(&app, first_req).await;
    assert_eq!(first_resp.status(), 204);

    let second_req = test::TestRequest::delete()
        .uri(&format!("/transactions/{id}"))
        .to_request();
    let second_resp = test::call_service(&app, second_req).await;
    assert_eq!(second_resp.status(), 404);
}
