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
async fn list_transactions_filters_by_search_matches_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA SUPERMARKT", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=starb")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_matches_category_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await; // category_id 1 = Other/Autre

    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": "2024-01-16",
            "merchant": "SCHOOL SUPPLIES",
            "amount": "40.00",
            "category_id": 7,
            "account": "User 1",
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=educ")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "SCHOOL SUPPLIES");
    assert_eq!(rows[0]["category_name_en"], "Education");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_matches_french_category_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": "2024-01-16",
            "merchant": "SCHOOL SUPPLIES",
            "amount": "40.00",
            "category_id": 7,
            "account": "User 1",
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=%C3%A9duc")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["category_name_fr"], "Éducation");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_matches_amount(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=12.34")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_is_case_insensitive(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=STARbucks")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn list_transactions_filters_by_search_treats_percent_literally(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "50% OFF STORE", "12.34").await;
    create_via_api(&app, "2024-01-16", "50X OFF STORE", "56.78").await;

    // "50% OFF" URL-encoded: %25 is a literal '%', %20 is a space.
    let req = test::TestRequest::get()
        .uri("/transactions?search=50%25%20OFF")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "50% OFF STORE");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_treats_underscore_literally(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "ITEM_CODE 123", "12.34").await;
    create_via_api(&app, "2024-01-16", "ITEMXCODE 123", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=ITEM_CODE")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], "ITEM_CODE 123");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_treats_backslash_literally(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", r"PATH\TO STORE", "12.34").await;

    // "PATH\TO" URL-encoded: %5C is a literal backslash.
    let req = test::TestRequest::get()
        .uri("/transactions?search=PATH%5CTO")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant"], r"PATH\TO STORE");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_returns_empty_when_no_matches(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=nonexistent")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
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

#[sqlx::test]
async fn list_transactions_orders_by_date_explicit(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=date")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    let dates: Vec<&str> = rows.iter().map(|r| r["date"].as_str().unwrap()).collect();
    assert_eq!(dates, vec!["2024-01-17", "2024-01-16", "2024-01-15"]);
}

#[sqlx::test]
async fn list_transactions_orders_by_inverse_date(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=inverse_date")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    let dates: Vec<&str> = rows.iter().map(|r| r["date"].as_str().unwrap()).collect();
    assert_eq!(dates, vec!["2024-01-15", "2024-01-16", "2024-01-17"]);
}

#[sqlx::test]
async fn list_transactions_orders_by_amount_desc(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=amount")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    let amounts: Vec<&str> = rows.iter().map(|r| r["amount"].as_str().unwrap()).collect();
    assert_eq!(amounts, vec!["56.78", "40.00", "12.34"]);
}

#[sqlx::test]
async fn list_transactions_orders_by_inverse_amount(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=inverse_amount")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    let amounts: Vec<&str> = rows.iter().map(|r| r["amount"].as_str().unwrap()).collect();
    assert_eq!(amounts, vec!["12.34", "40.00", "56.78"]);
}

#[sqlx::test]
async fn list_transactions_rejects_invalid_order(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=nonsense")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- GET /transactions/download ---

#[sqlx::test]
async fn download_transactions_csv_contains_header_and_rows(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "text/csv; charset=utf-8"
    );
    assert_eq!(
        resp.headers().get("content-disposition").unwrap(),
        "attachment; filename=\"transactions.csv\""
    );
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    let mut lines = csv.lines();
    assert_eq!(lines.next().unwrap(), "Date,Merchant,Category,Amount");
    assert_eq!(lines.next().unwrap(), "2024-01-16,IGA,Other,56.78");
    assert_eq!(lines.next().unwrap(), "2024-01-15,STARBUCKS,Other,12.34");
}

#[sqlx::test]
async fn download_transactions_filename_reflects_search_and_order(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&search=Coffee%20Shop!&order=amount")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-disposition").unwrap(),
        "attachment; filename=\"transactions_coffee-shop_highest-amount.csv\""
    );
}

#[sqlx::test]
async fn download_transactions_xlsx_returns_xlsx_content_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=xlsx")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    let body = test::read_body(resp).await;
    // .xlsx files are zip archives, which always start with the "PK" magic bytes.
    assert_eq!(&body[0..2], b"PK");
}

#[sqlx::test]
async fn download_transactions_respects_search_filter(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&search=starb")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    let mut lines = csv.lines();
    assert_eq!(lines.next().unwrap(), "Date,Merchant,Category,Amount");
    assert_eq!(lines.next().unwrap(), "2024-01-15,STARBUCKS,Other,12.34");
    assert!(lines.next().is_none());
}

#[sqlx::test]
async fn download_transactions_respects_order(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&order=inverse_date")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    let mut lines = csv.lines().skip(1);
    assert_eq!(lines.next().unwrap(), "2024-01-15,STARBUCKS,Other,12.34");
    assert_eq!(lines.next().unwrap(), "2024-01-16,IGA,Other,56.78");
}

#[sqlx::test]
async fn download_transactions_rejects_missing_format(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get()
        .uri("/transactions/download")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn download_transactions_rejects_invalid_format(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=pdf")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
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
    assert_eq!(body["category_name_en"], "Other");
    assert_eq!(body["category_name_fr"], "Autre");
    assert_eq!(body["category_type"], "expense");
}

#[sqlx::test]
async fn get_transaction_reflects_renamed_category(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(web::Data::new(L10n::new()))
            .configure(configure)
            .configure(crate::features::categories::configure),
    )
    .await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let patch_req = test::TestRequest::patch()
        .uri("/categories/1")
        .set_json(serde_json::json!({ "name_en": "Miscellaneous", "name_fr": "Divers" }))
        .to_request();
    let patch_resp = test::call_service(&app, patch_req).await;
    assert_eq!(patch_resp.status(), 200);

    let get_req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;

    assert_eq!(get_resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(get_resp).await;
    assert_eq!(body["category_name_en"], "Miscellaneous");
    assert_eq!(body["category_name_fr"], "Divers");
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
    assert_eq!(body["category_name_en"], "Other");
    assert_eq!(body["category_name_fr"], "Autre");
    assert_eq!(body["category_type"], "expense");
    assert_eq!(
        location,
        format!("/transactions/{}", body["id"].as_i64().unwrap())
    );
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
    assert_eq!(body["category_name_en"], "Education");
    assert_eq!(body["category_name_fr"], "Éducation");
    assert_eq!(body["category_type"], "expense");
    assert_eq!(body["account"], "User 2");
}

#[sqlx::test]
async fn create_transaction_rejects_unknown_category_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": 999999,
            "account": "User 1",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
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
async fn update_transaction_changes_category(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .set_json(serde_json::json!({ "category_id": 7 }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["category_id"], 7);
    assert_eq!(body["category_name_en"], "Education");
    assert_eq!(body["category_type"], "expense");
}

#[sqlx::test]
async fn update_transaction_rejects_unknown_category_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app, "2024-01-15", "STARBUCKS", "12.34").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .set_json(serde_json::json!({ "category_id": 999999 }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
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
