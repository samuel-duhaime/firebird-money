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
use crate::features::auth;
use crate::shared::anthropic::AnthropicClient;
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
    app_with_anthropic(
        pool,
        AnthropicClient::new(None, "claude-haiku-4-5".to_string()),
    )
}

/// Like `app_with`, with a given Claude API client — tests never reach the real API, so this is
/// only ever an unconfigured one or one with a fake key that's never used.
fn app_with_anthropic(
    pool: PgPool,
    anthropic: AnthropicClient,
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
        .app_data(web::Data::new(anthropic))
        .configure(configure)
        .configure(auth::configure)
        .configure(crate::features::category_groups::configure)
        .configure(crate::features::categories::configure)
        .configure(crate::features::tags::configure)
        .configure(crate::features::institutions::configure)
        .configure(crate::features::accounts::configure)
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
        .set_json(serde_json::json!({ "first_name": "Test", "last_name": "User" }))
        .to_request();
    assert_eq!(test::call_service(app, onboard_req).await.status(), 201);

    cookie
}

/// The signed-in caller's user id, per `GET /auth/me`.
async fn own_user_id<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body["user"]["id"].as_i64().unwrap()
}

async fn create_group_via_api<S, B>(app: &S, cookie: &str, name_en: &str, name_fr: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/category-groups")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name_en": name_en, "name_fr": name_fr, "type": "expense" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let status = resp.status();
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created group, status={status} body={body}"))
}

async fn create_category_via_api<S, B>(
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
        .set_json(
            serde_json::json!({ "group_id": group_id, "name_en": name_en, "name_fr": name_fr }),
        )
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created category, got {body}"))
}

/// A category named "Other"/"Autre" (expense) — matches what the old flat default seed called
/// `category_id: 1`, so most transaction assertions below didn't need to change once categories
/// became per-household. The group itself is named distinctly from the seeded "Other" group
/// (group names are unique per household; "Other" is already taken by a default group), but the
/// category name "Other" isn't used by any seeded leaf category, so it's free to create.
async fn create_other_category<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let group_id = create_group_via_api(app, cookie, "Custom Group", "Groupe personnalisé").await;
    create_category_via_api(app, cookie, group_id, "Other", "Autre").await
}

/// The id of the seeded "Education"/"Éducation" category (under the seeded "Education" group) —
/// fetched rather than created, since both that group and category names are already taken by the
/// household's starter data.
async fn find_category_id_by_name<S, B>(app: &S, cookie: &str, name_en: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/categories")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body.as_array()
        .unwrap()
        .iter()
        .find(|category| category["name_en"] == name_en)
        .unwrap_or_else(|| panic!("expected a seeded category named {name_en}"))["id"]
        .as_i64()
        .unwrap()
}

async fn education_category_id<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    find_category_id_by_name(app, cookie, "Education").await
}

/// Creates a transaction through `POST /transactions` (sending `original_statement`, so the
/// server's own matching/creation logic resolves a merchant — no common merchants exist in a
/// fresh test database, so this always creates a new custom one named exactly `statement`,
/// idempotently reused by a later call with the same text) and returns its id, for tests that only
/// need an existing row to act on.
async fn create_via_api<S, B>(
    app: &S,
    cookie: &str,
    category_id: i64,
    date: &str,
    statement: &str,
    amount: &str,
) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let account_id = crate::shared::test_support::test_account_id(app, cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": date,
            "original_statement": statement,
            "amount": amount,
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created transaction, got {body}"))
}

/// A transaction's current `merchant_id`, per `GET /transactions/{id}`.
async fn transaction_merchant_id<S, B>(app: &S, cookie: &str, id: i64) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body["merchant_id"].as_i64().unwrap()
}

/// Creates a tag through `POST /tags` and returns its id, for tests that need an existing tag to
/// attach to a transaction.
async fn create_tag_via_api<S, B>(app: &S, cookie: &str, name: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/tags")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": name, "color": "#2F80ED" }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created tag, got {body}"))
}

// --- GET /transactions ---

#[sqlx::test]
async fn list_transactions_returns_all_rows(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[sqlx::test]
async fn list_transactions_includes_tags(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;
    let tag_b_id = create_tag_via_api(&app, &cookie, "Test Tag B").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_a_id, tag_b_id],
        }))
        .to_request();
    assert_eq!(test::call_service(&app, create_req).await.status(), 201);
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    let starbucks = rows
        .iter()
        .find(|row| row["merchant_name"] == "STARBUCKS")
        .unwrap();
    let tag_names: Vec<&str> = starbucks["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["name"].as_str().unwrap())
        .collect();
    assert_eq!(tag_names, vec!["Test Tag A", "Test Tag B"]);

    let iga = rows
        .iter()
        .find(|row| row["merchant_name"] == "IGA")
        .unwrap();
    assert_eq!(iga["tags"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn list_transactions_search_does_not_match_tag_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let vacation_id = create_tag_via_api(&app, &cookie, "Vacation 2026").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "AIR CANADA",
            "amount": "250.00",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [vacation_id],
        }))
        .to_request();
    assert_eq!(test::call_service(&app, create_req).await.status(), 201);
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=vacation")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn list_transactions_search_matches_original_statement(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS STORE #4521",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    // "#4521" only appears in the raw statement, not in the matched merchant's name.
    let req = test::TestRequest::get()
        .uri("/transactions?search=%234521")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"].as_i64().unwrap(), id);
}

#[sqlx::test]
async fn list_transactions_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get().uri("/transactions").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn list_transactions_does_not_leak_another_households_transactions(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    create_via_api(
        &app,
        &cookie_a,
        category_a,
        "2024-01-15",
        "ONLY A'S",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions")
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn list_transactions_filters_by_merchant_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    let iga_transaction_id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-16",
        "IGA SUPERMARKT",
        "56.78",
    )
    .await;
    let iga_merchant_id = transaction_merchant_id(&app, &cookie, iga_transaction_id).await;

    let req = test::TestRequest::get()
        .uri(&format!("/transactions?merchant_id={iga_merchant_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "IGA SUPERMARKT");
}

#[sqlx::test]
async fn list_transactions_filters_by_date(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?date=2024-01-15")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_date_and_merchant_id_combined(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    let iga_jan_15 = create_via_api(&app, &cookie, category_id, "2024-01-15", "IGA", "56.78").await;
    // Same raw text as the transaction above — resolve_or_create matches the merchant it already
    // created rather than making a second "IGA" row, so both transactions share one merchant_id.
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "78.90").await;
    let iga_merchant_id = transaction_merchant_id(&app, &cookie, iga_jan_15).await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/transactions?date=2024-01-15&merchant_id={iga_merchant_id}"
        ))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "IGA");
    assert_eq!(rows[0]["date"], "2024-01-15");
}

#[sqlx::test]
async fn list_transactions_filters_by_date_range(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-14",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-15", "IGA", "56.78").await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "METRO", "20.00").await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "COSTCO", "99.99").await;

    let req = test::TestRequest::get()
        .uri("/transactions?start_date=2024-01-15&end_date=2024-01-16")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    let dates: Vec<&str> = rows.iter().map(|r| r["date"].as_str().unwrap()).collect();
    assert_eq!(dates, vec!["2024-01-16", "2024-01-15"]);
}

#[sqlx::test]
async fn list_transactions_filters_by_start_date_only(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?start_date=2024-01-16")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "IGA");
}

#[sqlx::test]
async fn list_transactions_filters_by_end_date_only(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?end_date=2024-01-15")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_matches_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-16",
        "IGA SUPERMARKT",
        "56.78",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=starb")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_matches_category_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let other_id = create_other_category(&app, &cookie).await;
    create_via_api(&app, &cookie, other_id, "2024-01-15", "STARBUCKS", "12.34").await;
    let education_id = education_category_id(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        education_id,
        "2024-01-16",
        "SCHOOL SUPPLIES",
        "40.00",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=educ")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "SCHOOL SUPPLIES");
    assert_eq!(rows[0]["category_name_en"], "Education");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_matches_french_category_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let education_id = education_category_id(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        education_id,
        "2024-01-16",
        "SCHOOL SUPPLIES",
        "40.00",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=%C3%A9duc")
        .insert_header(("Cookie", cookie))
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=12.34")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_is_case_insensitive(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=STARbucks")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn list_transactions_filters_by_search_treats_percent_literally(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "50% OFF STORE",
        "12.34",
    )
    .await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-16",
        "50X OFF STORE",
        "56.78",
    )
    .await;

    // "50% OFF" URL-encoded: %25 is a literal '%', %20 is a space.
    let req = test::TestRequest::get()
        .uri("/transactions?search=50%25%20OFF")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "50% OFF STORE");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_treats_underscore_literally(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "ITEM_CODE 123",
        "12.34",
    )
    .await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-16",
        "ITEMXCODE 123",
        "56.78",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=ITEM_CODE")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], "ITEM_CODE 123");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_treats_backslash_literally(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        r"PATH\TO STORE",
        "12.34",
    )
    .await;

    // "PATH\TO" URL-encoded: %5C is a literal backslash.
    let req = test::TestRequest::get()
        .uri("/transactions?search=PATH%5CTO")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["merchant_name"], r"PATH\TO STORE");
}

#[sqlx::test]
async fn list_transactions_filters_by_search_returns_empty_when_no_matches(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?search=nonexistent")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn list_transactions_returns_empty_array_when_no_matches(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions?merchant_id=999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn list_transactions_orders_by_date_desc(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .to_request();
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=date")
        .insert_header(("Cookie", cookie))
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=inverse_date")
        .insert_header(("Cookie", cookie))
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=amount")
        .insert_header(("Cookie", cookie))
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "SHELL", "40.00").await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=inverse_amount")
        .insert_header(("Cookie", cookie))
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/transactions?order=nonsense")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

/// `GET {uri}`'s rows' merchant names, in response order.
async fn listed_merchant_names<S, B>(app: &S, cookie: &str, uri: &str) -> Vec<String>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri(uri)
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body.as_array()
        .unwrap_or_else(|| panic!("expected an array, got {body}"))
        .iter()
        .map(|row| row["merchant_name"].as_str().unwrap().to_string())
        .collect()
}

#[sqlx::test]
async fn list_transactions_filters_by_category_ids(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let other_id = create_other_category(&app, &cookie).await;
    let education_id = education_category_id(&app, &cookie).await;
    let groceries_id = find_category_id_by_name(&app, &cookie, "Groceries").await;
    create_via_api(&app, &cookie, other_id, "2024-01-15", "STARBUCKS", "12.34").await;
    create_via_api(&app, &cookie, education_id, "2024-01-16", "UDEMY", "20.00").await;
    create_via_api(&app, &cookie, groceries_id, "2024-01-17", "IGA", "56.78").await;

    let names = listed_merchant_names(
        &app,
        &cookie,
        &format!("/transactions?category_ids={other_id},{groceries_id}"),
    )
    .await;

    assert_eq!(names, vec!["IGA", "STARBUCKS"]);
}

#[sqlx::test]
async fn list_transactions_filters_by_merchant_ids(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let starbucks =
        create_via_api(&app, &cookie, category_id, "2024-01-15", "STARBUCKS", "1").await;
    let iga = create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "2").await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "METRO", "3").await;
    let starbucks_id = transaction_merchant_id(&app, &cookie, starbucks).await;
    let iga_id = transaction_merchant_id(&app, &cookie, iga).await;

    let names = listed_merchant_names(
        &app,
        &cookie,
        &format!("/transactions?merchant_ids={starbucks_id},{iga_id}"),
    )
    .await;

    assert_eq!(names, vec!["IGA", "STARBUCKS"]);
}

#[sqlx::test]
async fn list_transactions_filters_by_account_ids(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(&app, &cookie, category_id, "2024-01-15", "STARBUCKS", "1").await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;

    let names = listed_merchant_names(
        &app,
        &cookie,
        &format!("/transactions?account_ids={account_id}"),
    )
    .await;
    assert_eq!(names, vec!["STARBUCKS"]);

    let names = listed_merchant_names(
        &app,
        &cookie,
        &format!("/transactions?account_ids={}", account_id + 1000),
    )
    .await;
    assert!(names.is_empty());
}

#[sqlx::test]
async fn list_transactions_filters_by_tag_ids_and_untagged(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;
    let tag_b_id = create_tag_via_api(&app, &cookie, "Test Tag B").await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;

    for (date, statement, tag_ids) in [
        ("2024-01-15", "STARBUCKS", vec![tag_a_id]),
        ("2024-01-16", "IGA", vec![tag_b_id]),
        ("2024-01-17", "METRO", vec![tag_a_id, tag_b_id]),
    ] {
        let req = test::TestRequest::post()
            .uri("/transactions")
            .insert_header(("Cookie", cookie.clone()))
            .set_json(serde_json::json!({
                "date": date,
                "original_statement": statement,
                "amount": "10.00",
                "category_id": category_id,
                "account_id": account_id,
                "tag_ids": tag_ids,
            }))
            .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 201);
    }
    create_via_api(&app, &cookie, category_id, "2024-01-18", "COSTCO", "10.00").await;

    let tagged =
        listed_merchant_names(&app, &cookie, &format!("/transactions?tag_ids={tag_a_id}")).await;
    assert_eq!(tagged, vec!["METRO", "STARBUCKS"]);

    let untagged = listed_merchant_names(&app, &cookie, "/transactions?untagged=true").await;
    assert_eq!(untagged, vec!["COSTCO"]);

    let both = listed_merchant_names(
        &app,
        &cookie,
        &format!("/transactions?tag_ids={tag_b_id}&untagged=true"),
    )
    .await;
    assert_eq!(both, vec!["COSTCO", "METRO", "IGA"]);
}

#[sqlx::test]
async fn list_transactions_filters_by_amount(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "5.00",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "10.00").await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "METRO", "50.00").await;

    let cases = [
        ("amount_op=greater_than&amount=10", vec!["METRO"]),
        ("amount_op=less_than&amount=10", vec!["STARBUCKS"]),
        ("amount_op=equal_to&amount=10", vec!["IGA"]),
        (
            "amount_op=between&amount=5&amount_max=10",
            vec!["IGA", "STARBUCKS"],
        ),
        // An operator with no amount isn't a filter yet.
        ("amount_op=greater_than", vec!["METRO", "IGA", "STARBUCKS"]),
    ];
    for (query, expected) in cases {
        let names = listed_merchant_names(&app, &cookie, &format!("/transactions?{query}")).await;
        assert_eq!(names, expected, "for {query}");
    }
}

#[sqlx::test]
async fn list_transactions_rejects_invalid_id_list(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/transactions?category_ids=1,abc")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- GET /transactions/download ---

#[sqlx::test]
async fn download_transactions_csv_contains_header_and_rows(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv")
        .insert_header(("Cookie", cookie))
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
    assert_eq!(
        lines.next().unwrap(),
        "Date,Merchant,Category,Tags,Account,Amount"
    );
    assert_eq!(
        lines.next().unwrap(),
        "2024-01-16,IGA,Other,,Test Account,56.78"
    );
    assert_eq!(
        lines.next().unwrap(),
        "2024-01-15,STARBUCKS,Other,,Test Account,12.34"
    );
}

#[sqlx::test]
async fn download_transactions_includes_tags_column(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;
    let tag_b_id = create_tag_via_api(&app, &cookie, "Test Tag B").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_a_id, tag_b_id],
        }))
        .to_request();
    assert_eq!(test::call_service(&app, create_req).await.status(), 201);

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(
        csv.lines().nth(1).unwrap(),
        "2024-01-15,STARBUCKS,Other,\"Test Tag A, Test Tag B\",Test Account,12.34"
    );
}

#[sqlx::test]
async fn download_transactions_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn download_transactions_does_not_leak_another_households_transactions(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    create_via_api(
        &app,
        &cookie_a,
        category_a,
        "2024-01-15",
        "ONLY A'S",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv")
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    assert!(!csv.contains("ONLY A'S"));
}

#[sqlx::test]
async fn download_transactions_csv_escapes_formula_like_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "=SUM(A1:A2)",
        "12.34",
    )
    .await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-16",
        "+1234567890",
        "20.00",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-17", "-2+3", "30.00").await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-18",
        "@SUM(A1)",
        "40.00",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&order=inverse_date")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    let rows: Vec<&str> = csv.lines().skip(1).collect();
    assert_eq!(rows[0], "2024-01-15,'=SUM(A1:A2),Other,,Test Account,12.34");
    assert_eq!(rows[1], "2024-01-16,'+1234567890,Other,,Test Account,20.00");
    assert_eq!(rows[2], "2024-01-17,'-2+3,Other,,Test Account,30.00");
    assert_eq!(rows[3], "2024-01-18,'@SUM(A1),Other,,Test Account,40.00");
}

#[sqlx::test]
async fn download_transactions_csv_escapes_formula_like_category(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let patch_req = test::TestRequest::patch()
        .uri(&format!("/categories/{category_id}"))
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({ "name_en": "=cmd" }))
        .to_request();
    assert_eq!(test::call_service(&app, patch_req).await.status(), 200);

    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(
        csv.lines().nth(1).unwrap(),
        "2024-01-15,STARBUCKS,'=cmd,,Test Account,12.34"
    );
}

#[sqlx::test]
async fn download_transactions_filename_reflects_search_and_order(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&search=Coffee%20Shop!&order=amount")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-disposition").unwrap(),
        "attachment; filename=\"transactions_coffee-shop_highest-amount.csv\""
    );
}

#[sqlx::test]
async fn download_transactions_filename_reflects_date_range(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&start_date=2024-01-01&end_date=2024-01-31")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("content-disposition").unwrap(),
        "attachment; filename=\"transactions_2024-01-01_to_2024-01-31.csv\""
    );
}

#[sqlx::test]
async fn download_transactions_respects_date_range_filter(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-02-15", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&start_date=2024-01-01&end_date=2024-01-31")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    assert!(csv.contains("STARBUCKS"));
    assert!(!csv.contains("IGA"));
}

#[sqlx::test]
async fn download_transactions_xlsx_returns_xlsx_content_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=xlsx")
        .insert_header(("Cookie", cookie))
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&search=starb")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    let mut lines = csv.lines();
    assert_eq!(
        lines.next().unwrap(),
        "Date,Merchant,Category,Tags,Account,Amount"
    );
    assert_eq!(
        lines.next().unwrap(),
        "2024-01-15,STARBUCKS,Other,,Test Account,12.34"
    );
    assert!(lines.next().is_none());
}

#[sqlx::test]
async fn download_transactions_respects_order(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=csv&order=inverse_date")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body = test::read_body(resp).await;
    let csv = String::from_utf8(body.to_vec()).unwrap();
    let mut lines = csv.lines().skip(1);
    assert_eq!(
        lines.next().unwrap(),
        "2024-01-15,STARBUCKS,Other,,Test Account,12.34"
    );
    assert_eq!(
        lines.next().unwrap(),
        "2024-01-16,IGA,Other,,Test Account,56.78"
    );
}

#[sqlx::test]
async fn download_transactions_rejects_missing_format(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn download_transactions_rejects_invalid_format(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/transactions/download?format=pdf")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- GET /transactions/{id} ---

#[sqlx::test]
async fn get_transaction_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["merchant_name"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["category_name_en"], "Other");
    assert_eq!(body["category_name_fr"], "Autre");
    assert_eq!(body["category_type"], "expense");
    assert!(body["household_id"].is_i64());
    assert!(body["user_id"].is_i64());
    assert_eq!(body["tags"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn get_transaction_includes_tags(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_id],
        }))
        .to_request();
    let create_body: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let id = create_body["id"].as_i64().unwrap();
    assert_eq!(create_body["tags"][0]["name"], "Test Tag A");

    let req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    let tags = body["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0]["id"], tag_id);
    assert_eq!(tags[0]["name"], "Test Tag A");
    assert_eq!(tags[0]["color"], "#2F80ED");
}

#[sqlx::test]
async fn get_transaction_reflects_renamed_category(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let patch_req = test::TestRequest::patch()
        .uri(&format!("/categories/{category_id}"))
        .insert_header(("Cookie", cookie.clone()))
        .set_json(
            serde_json::json!({ "name_en": "Renamed Category", "name_fr": "Catégorie renommée" }),
        )
        .to_request();
    let patch_resp = test::call_service(&app, patch_req).await;
    assert_eq!(patch_resp.status(), 200);

    let get_req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;

    assert_eq!(get_resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(get_resp).await;
    assert_eq!(body["category_name_en"], "Renamed Category");
    assert_eq!(body["category_name_fr"], "Catégorie renommée");
}

#[sqlx::test]
async fn get_transaction_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/transactions/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn get_transaction_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    let id = create_via_api(
        &app,
        &cookie_a,
        category_a,
        "2024-01-15",
        "ONLY A'S",
        "12.34",
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- POST /transactions ---

#[sqlx::test]
async fn create_transaction_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
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
    assert_eq!(body["merchant_name"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["category_name_en"], "Other");
    assert_eq!(body["category_name_fr"], "Autre");
    assert_eq!(body["category_type"], "expense");
    assert_eq!(body["source"], "manual");
    assert_eq!(
        location,
        format!("/transactions/{}", body["id"].as_i64().unwrap())
    );
    assert_eq!(body["tags"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn create_transaction_persists_original_statement_separately_from_merchant_name(
    pool: PgPool,
) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS STORE #4521",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    // No existing merchant to match (a fresh test database seeds no common merchants — see
    // merchants::repository::seed_defaults), so a new custom one is created named exactly the raw
    // statement text, and merchant_name/original_statement are identical here. The two fields only
    // diverge once an existing merchant is matched or the merchant is later renamed/repointed.
    assert_eq!(body["original_statement"], "STARBUCKS STORE #4521");
    assert_eq!(body["merchant_name"], "STARBUCKS STORE #4521");
    assert!(body["merchant_id"].is_i64());
}

#[sqlx::test]
async fn create_transaction_reuses_a_merchant_matched_from_a_second_statement(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let first_id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "Starbucks",
        "12.34",
    )
    .await;
    let first_merchant_id = transaction_merchant_id(&app, &cookie, first_id).await;

    // A second, differently-formatted statement containing the same merchant name should match
    // the custom merchant the first transaction just created, not create a second "Starbucks".
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-16",
            "original_statement": "STARBUCKS STORE #4521",
            "amount": "5.00",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["merchant_id"].as_i64().unwrap(), first_merchant_id);
    assert_eq!(body["merchant_name"], "Starbucks");
    assert_eq!(body["original_statement"], "STARBUCKS STORE #4521");
}

#[sqlx::test]
async fn create_transaction_matching_requires_a_word_boundary(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let td_id = create_via_api(&app, &cookie, category_id, "2024-01-15", "TD", "12.34").await;
    let td_merchant_id = transaction_merchant_id(&app, &cookie, td_id).await;

    // "LTD" contains "TD" as a raw substring, but not at a word boundary, so it must not match the
    // "TD" merchant — it should fall back to creating its own new custom merchant instead.
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-16",
            "original_statement": "PAYMENT TO LTD COMPANY",
            "amount": "5.00",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_ne!(body["merchant_id"].as_i64().unwrap(), td_merchant_id);
    assert_eq!(body["merchant_name"], "PAYMENT TO LTD COMPANY");

    // "MY TD BANK VISA" does have "TD" at a word boundary, so it should match the existing merchant.
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-17",
            "original_statement": "MY TD BANK VISA",
            "amount": "5.00",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["merchant_id"].as_i64().unwrap(), td_merchant_id);
    assert_eq!(body["merchant_name"], "TD");
}

#[sqlx::test]
async fn create_transaction_with_explicit_merchant_id_defaults_original_statement_to_its_name(
    pool: PgPool,
) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let first_id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "Starbucks",
        "12.34",
    )
    .await;
    let merchant_id = transaction_merchant_id(&app, &cookie, first_id).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    // The manual "Add Transaction" shape: merchant_id picked explicitly, no original_statement —
    // there's no raw bank text for a manually-entered row, so the server falls back to the
    // merchant's own name.
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-16",
            "merchant_id": merchant_id,
            "amount": "7.00",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["merchant_id"].as_i64().unwrap(), merchant_id);
    assert_eq!(body["merchant_name"], "Starbucks");
    assert_eq!(body["original_statement"], "Starbucks");
}

#[sqlx::test]
async fn create_transaction_rejects_an_unknown_merchant_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant_id": 999999,
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn create_transaction_rejects_another_households_merchant_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    let category_b = create_other_category(&app, &cookie_b).await;
    let id_b = create_via_api(
        &app,
        &cookie_b,
        category_b,
        "2024-01-15",
        "ONLY B'S",
        "56.78",
    )
    .await;
    let merchant_b_id = transaction_merchant_id(&app, &cookie_b, id_b).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie_a).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie_a))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant_id": merchant_b_id,
            "amount": "12.34",
            "category_id": category_a,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_transaction_requires_a_merchant_id_or_original_statement(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn create_transaction_with_tag_ids_attaches_tags(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;
    let tag_b_id = create_tag_via_api(&app, &cookie, "Test Tag B").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_a_id, tag_b_id],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let tag_ids: Vec<i64> = body["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["id"].as_i64().unwrap())
        .collect();
    assert_eq!(tag_ids, vec![tag_a_id, tag_b_id]);
}

#[sqlx::test]
async fn create_transaction_ignores_tag_ids_from_another_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    let tag_b = create_tag_via_api(&app, &cookie_b, "Only B's Tag").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie_a).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie_a))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_a,
            "account_id": account_id,
            "tag_ids": [tag_b],
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["tags"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn create_transaction_attributes_it_to_the_caller(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let expected_user_id = own_user_id(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["user_id"].as_i64().unwrap(), expected_user_id);
}

#[sqlx::test]
async fn create_transaction_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": 1,
            "account_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn create_transaction_requires_a_household(pool: PgPool) {
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
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": 1,
            "account_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 403);
}

#[sqlx::test]
async fn create_transaction_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "original_statement": "STARBUCKS" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_transaction_persists_all_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = education_category_id(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["date"], "2024-01-15");
    assert_eq!(body["merchant_name"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["category_id"].as_i64().unwrap(), category_id);
    assert_eq!(body["category_name_en"], "Education");
    assert_eq!(body["category_name_fr"], "Éducation");
    assert_eq!(body["category_type"], "expense");
    assert_eq!(body["account_id"].as_i64().unwrap(), account_id);
    assert_eq!(body["account_name"], "Test Account");
}

#[sqlx::test]
async fn create_transaction_defaults_reviewed_to_true(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["reviewed"], true);
}

#[sqlx::test]
async fn create_transaction_respects_explicit_reviewed_false(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "reviewed": false,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["reviewed"], false);
}

#[sqlx::test]
async fn create_transaction_rejects_unknown_category_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": 999999,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn create_transaction_rejects_another_households_category(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie_b).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_a,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_transaction_rejects_invalid_date(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "not-a-date",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- PATCH /transactions/{id} ---

#[sqlx::test]
async fn update_transaction_changes_only_given_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "amount": "20.00" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["amount"], "20.00");
    assert_eq!(body["merchant_name"], "STARBUCKS");
}

#[sqlx::test]
async fn update_transaction_changes_category(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let other_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(&app, &cookie, other_id, "2024-01-15", "STARBUCKS", "12.34").await;
    let education_id = education_category_id(&app, &cookie).await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "category_id": education_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["category_id"].as_i64().unwrap(), education_id);
    assert_eq!(body["category_name_en"], "Education");
    assert_eq!(body["category_type"], "expense");
}

#[sqlx::test]
async fn update_transaction_changes_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    let iga_id = create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;
    let iga_merchant_id = transaction_merchant_id(&app, &cookie, iga_id).await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "merchant_id": iga_merchant_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["merchant_id"].as_i64().unwrap(), iga_merchant_id);
    assert_eq!(body["merchant_name"], "IGA");
    // original_statement is immutable — changing merchant_id never touches it.
    assert_eq!(body["original_statement"], "STARBUCKS");
}

#[sqlx::test]
async fn update_transaction_rejects_an_unknown_merchant_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "merchant_id": 999999 }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn update_transaction_rejects_another_households_merchant_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    let category_b = create_other_category(&app, &cookie_b).await;
    let id_a = create_via_api(
        &app,
        &cookie_a,
        category_a,
        "2024-01-15",
        "ONLY A'S",
        "12.34",
    )
    .await;
    let id_b = create_via_api(
        &app,
        &cookie_b,
        category_b,
        "2024-01-15",
        "ONLY B'S",
        "56.78",
    )
    .await;
    let merchant_b_id = transaction_merchant_id(&app, &cookie_b, id_b).await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id_a}"))
        .insert_header(("Cookie", cookie_a))
        .set_json(serde_json::json!({ "merchant_id": merchant_b_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn update_transaction_replaces_tags(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;
    let tag_b_id = create_tag_via_api(&app, &cookie, "Test Tag B").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_a_id],
        }))
        .to_request();
    let create_body: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let id = create_body["id"].as_i64().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "tag_ids": [tag_b_id] }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let tag_ids: Vec<i64> = body["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["id"].as_i64().unwrap())
        .collect();
    assert_eq!(tag_ids, vec![tag_b_id]);
}

#[sqlx::test]
async fn update_transaction_clears_tags_with_empty_array(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_a_id],
        }))
        .to_request();
    let create_body: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let id = create_body["id"].as_i64().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "tag_ids": [] }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["tags"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn update_transaction_without_tag_ids_leaves_tags_unchanged(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;

    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_a_id],
        }))
        .to_request();
    let create_body: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let id = create_body["id"].as_i64().unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "amount": "20.00" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["tags"].as_array().unwrap().len(), 1);
    assert_eq!(body["tags"][0]["name"], "Test Tag A");
}

#[sqlx::test]
async fn update_transaction_ignores_tag_ids_from_another_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    let id = create_via_api(
        &app,
        &cookie_a,
        category_a,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    let tag_b = create_tag_via_api(&app, &cookie_b, "Only B's Tag").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie_a))
        .set_json(serde_json::json!({ "tag_ids": [tag_b] }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["tags"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn update_transaction_rejects_unknown_category_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
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
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::patch()
        .uri("/transactions/999999")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "amount": "20.00" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn update_transaction_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    let id = create_via_api(
        &app,
        &cookie_a,
        category_a,
        "2024-01-15",
        "ONLY A'S",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "amount": "999.00" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn update_transaction_with_empty_body_leaves_row_unchanged(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["merchant_name"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["date"], "2024-01-15");
}

#[sqlx::test]
async fn update_transaction_rejects_malformed_body(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "amount": true }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- PATCH /transactions/bulk ---

#[sqlx::test]
async fn bulk_update_transactions_adds_tags_without_removing_existing(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let tag_a_id = create_tag_via_api(&app, &cookie, "Test Tag A").await;
    let tag_b_id = create_tag_via_api(&app, &cookie, "Test Tag B").await;

    // One transaction already carries a tag — the bulk add below must keep it, not replace it.
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let create_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": account_id,
            "tag_ids": [tag_a_id],
        }))
        .to_request();
    let starbucks_id: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let starbucks_id = starbucks_id["id"].as_i64().unwrap();
    let iga_id = create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "56.78").await;

    let req = test::TestRequest::patch()
        .uri("/transactions/bulk")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "ids": [starbucks_id, iga_id],
            "patch": { "tag_ids": [tag_b_id] },
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();

    let starbucks = rows
        .iter()
        .find(|row| row["id"].as_i64() == Some(starbucks_id))
        .unwrap();
    let mut starbucks_tags: Vec<&str> = starbucks["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["name"].as_str().unwrap())
        .collect();
    starbucks_tags.sort_unstable();
    assert_eq!(starbucks_tags, vec!["Test Tag A", "Test Tag B"]);

    let iga = rows
        .iter()
        .find(|row| row["id"].as_i64() == Some(iga_id))
        .unwrap();
    let iga_tags: Vec<&str> = iga["tags"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tag| tag["name"].as_str().unwrap())
        .collect();
    assert_eq!(iga_tags, vec!["Test Tag B"]);
}

#[sqlx::test]
async fn bulk_update_transactions_skips_ids_the_household_doesnt_own_for_tags(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_b = create_other_category(&app, &cookie_b).await;
    let tag_a = create_tag_via_api(&app, &cookie_a, "Test Tag A").await;
    let only_bs_id = create_via_api(
        &app,
        &cookie_b,
        category_b,
        "2024-01-15",
        "ONLY B'S",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri("/transactions/bulk")
        .insert_header(("Cookie", cookie_a))
        .set_json(serde_json::json!({
            "ids": [only_bs_id],
            "patch": { "tag_ids": [tag_a] },
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);

    // Confirm nothing was attached to B's transaction despite the (ignored) request.
    let get_req = test::TestRequest::get()
        .uri(&format!("/transactions/{only_bs_id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let get_body: serde_json::Value = test::call_and_read_body_json(&app, get_req).await;
    assert_eq!(get_body["tags"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn bulk_update_transactions_changes_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let starbucks_id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    let costco_id =
        create_via_api(&app, &cookie, category_id, "2024-01-16", "COSTCO", "99.99").await;
    let iga_id = create_via_api(&app, &cookie, category_id, "2024-01-17", "IGA", "56.78").await;
    let iga_merchant_id = transaction_merchant_id(&app, &cookie, iga_id).await;

    let req = test::TestRequest::patch()
        .uri("/transactions/bulk")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "ids": [starbucks_id, costco_id],
            "patch": { "merchant_id": iga_merchant_id },
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        assert_eq!(row["merchant_id"].as_i64().unwrap(), iga_merchant_id);
        assert_eq!(row["merchant_name"], "IGA");
    }

    // The third transaction, left out of `ids`, keeps its own merchant.
    let unaffected_merchant_id = transaction_merchant_id(&app, &cookie, iga_id).await;
    assert_eq!(unaffected_merchant_id, iga_merchant_id);
}

#[sqlx::test]
async fn bulk_update_transactions_rejects_an_unknown_merchant_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri("/transactions/bulk")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "ids": [id],
            "patch": { "merchant_id": 999999 },
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- DELETE /transactions/{id} ---

#[sqlx::test]
async fn delete_transaction_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_transaction_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;
    let id = create_via_api(
        &app,
        &cookie_a,
        category_a,
        "2024-01-15",
        "ONLY A'S",
        "12.34",
    )
    .await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 404);
}

#[sqlx::test]
async fn delete_transaction_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/transactions/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_transaction_twice_returns_not_found_second_time(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let first_req = test::TestRequest::delete()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let first_resp = test::call_service(&app, first_req).await;
    assert_eq!(first_resp.status(), 204);

    let second_req = test::TestRequest::delete()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let second_resp = test::call_service(&app, second_req).await;
    assert_eq!(second_resp.status(), 404);
}

// --- account_id ---

/// Creates a second account (in its own custom institution) and returns its id.
async fn create_second_account<S, B>(app: &S, cookie: &str, name: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/institutions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": format!("{name} Bank") }))
        .to_request();
    let institution: serde_json::Value = test::call_and_read_body_json(app, req).await;

    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "institution_id": institution["id"],
            "name": name,
            "type": "savings",
        }))
        .to_request();
    let account: serde_json::Value = test::call_and_read_body_json(app, req).await;
    account["id"].as_i64().unwrap()
}

#[sqlx::test]
async fn create_transaction_rejects_another_households_account(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let foreign_account_id = crate::shared::test_support::test_account_id(&app, &cookie_a).await;
    let category_id = create_other_category(&app, &cookie_b).await;

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "original_statement": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account_id": foreign_account_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("account"));
}

#[sqlx::test]
async fn update_transaction_moves_it_to_another_account(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let id = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;
    let savings_id = create_second_account(&app, &cookie, "Savings").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "account_id": savings_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["account_id"].as_i64().unwrap(), savings_id);
    assert_eq!(body["account_name"], "Savings");
}

#[sqlx::test]
async fn update_transaction_rejects_another_households_account(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let foreign_account_id = crate::shared::test_support::test_account_id(&app, &cookie_a).await;
    let category_id = create_other_category(&app, &cookie_b).await;
    let id = create_via_api(
        &app,
        &cookie_b,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "12.34",
    )
    .await;

    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "account_id": foreign_account_id }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("account"));
}

#[sqlx::test]
async fn bulk_update_transactions_moves_them_to_another_account(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let first = create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "1.00",
    )
    .await;
    let second = create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "2.00").await;
    let savings_id = create_second_account(&app, &cookie, "Savings").await;

    let req = test::TestRequest::patch()
        .uri("/transactions/bulk")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "ids": [first, second],
            "patch": { "account_id": savings_id },
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows
        .iter()
        .all(|row| row["account_id"].as_i64() == Some(savings_id)));
}

#[sqlx::test]
async fn list_transactions_filters_by_account_id_but_search_ignores_account_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    create_via_api(
        &app,
        &cookie,
        category_id,
        "2024-01-15",
        "STARBUCKS",
        "1.00",
    )
    .await;
    let moved = create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "2.00").await;
    let savings_id = create_second_account(&app, &cookie, "Vacances").await;
    let req = test::TestRequest::patch()
        .uri(&format!("/transactions/{moved}"))
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({ "account_id": savings_id }))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 200);

    let req = test::TestRequest::get()
        .uri(&format!("/transactions?account_id={savings_id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"].as_i64().unwrap(), moved);

    let req = test::TestRequest::get()
        .uri("/transactions?search=vacances")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// --- POST /transactions/import/read ---
//
// Only what's answered before Claude is called: tests never reach the real API. With a fake key,
// every request below is rejected up front, so the key is never sent anywhere.

/// A client with a key that's never used — see the note above.
fn fake_anthropic() -> AnthropicClient {
    AnthropicClient::new(Some("test-key".to_string()), "claude-haiku-4-5".to_string())
}

/// A multipart `POST /transactions/import/read` with `file` (and `account_id`, if given).
fn read_statement_request(file: &[u8], account_id: Option<&str>) -> test::TestRequest {
    const BOUNDARY: &str = "statement-boundary";
    let mut body = Vec::new();
    if let Some(account_id) = account_id {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"account_id\"\r\n\r\n{account_id}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"statement\"\r\n\
             Content-Type: application/octet-stream\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());

    test::TestRequest::post()
        .uri("/transactions/import/read")
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        ))
        .set_payload(body)
}

#[sqlx::test]
async fn read_statement_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with_anthropic(pool, fake_anthropic())).await;

    let req = read_statement_request(b"%PDF-1.7", None).to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn read_statement_is_unavailable_without_an_api_key(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = read_statement_request(b"%PDF-1.7", None)
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 503);
}

#[sqlx::test]
async fn read_statement_rejects_a_file_that_is_not_a_pdf_or_image(pool: PgPool) {
    let app = test::init_service(app_with_anthropic(pool, fake_anthropic())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    // A `.pdf` name or content type wouldn't help: the file's own bytes decide.
    let req = read_statement_request(b"\x00\x01\x02\x03 not a statement", None)
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("PDF"));
}

#[sqlx::test]
async fn read_statement_rejects_a_csv_too_long_to_read_at_once(pool: PgPool) {
    let app = test::init_service(app_with_anthropic(pool, fake_anthropic())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    // Well under the 10 MB upload limit, but over what one read accepts.
    let csv = "2026-09-02,IGA EXTRA #8123,-142.37\n".repeat(10_000);
    let req = read_statement_request(csv.as_bytes(), None)
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("too long"));
}

#[sqlx::test]
async fn read_statement_rejects_another_households_account(pool: PgPool) {
    let app = test::init_service(app_with_anthropic(pool, fake_anthropic())).await;
    let other_cookie = sign_in_with_household(&app, "other@example.com").await;
    let other_account_id = crate::shared::test_support::test_account_id(&app, &other_cookie).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = read_statement_request(b"%PDF-1.7", Some(&other_account_id.to_string()))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn read_statement_rejects_a_file_over_the_size_limit(pool: PgPool) {
    let app = test::init_service(app_with_anthropic(pool, fake_anthropic())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let mut oversized = b"%PDF-1.7".to_vec();
    oversized.resize(11 * 1024 * 1024, b'a'); // over the 10 MB field limit
    let req = read_statement_request(&oversized, None)
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    // The field-level size limit surfaces as `MultipartError::Payload(PayloadError::Overflow)`,
    // easy to mis-map to the wrong Fluent key since it isn't `MultipartError::Field`.
    assert!(body["error"].as_str().unwrap().contains("too large"));
}

// --- POST /transactions/import ---

/// `POST /transactions/import` with `lines` as the `transactions` array.
async fn import_via_api<S, B>(app: &S, cookie: &str, lines: serde_json::Value) -> ServiceResponse<B>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/transactions/import")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "transactions": lines }))
        .to_request();
    test::call_service(app, req).await
}

/// Every transaction the caller can see, per `GET /transactions` (most recent first).
async fn list_all<S, B>(app: &S, cookie: &str) -> Vec<serde_json::Value>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body.as_array().unwrap().clone()
}

#[sqlx::test]
async fn import_transactions_adds_every_line_as_imported(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let tag_id = create_tag_via_api(&app, &cookie, "Shared").await;
    // An existing merchant, created by hand first.
    let existing_id =
        create_via_api(&app, &cookie, category_id, "2026-09-01", "IGA", "10.00").await;
    let iga_id = transaction_merchant_id(&app, &cookie, existing_id).await;

    let resp = import_via_api(
        &app,
        &cookie,
        serde_json::json!([
            {
                "date": "2026-09-02",
                "original_statement": "IGA EXTRA #8123",
                "amount": "142.37",
                "merchant_id": iga_id,
                "category_id": category_id,
                "account_id": account_id,
                "tag_ids": [tag_id]
            },
            {
                "date": "2026-09-14",
                "original_statement": "CAFE OLIMPICO MTL",
                "amount": "9.40",
                "new_merchant_name": "Café Olimpico",
                "category_id": category_id,
                "account_id": account_id
            }
        ]),
    )
    .await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["created_count"], 2);
    assert_eq!(body["created_merchant_count"], 1);

    let transactions = list_all(&app, &cookie).await;
    assert_eq!(transactions.len(), 3);
    let cafe = &transactions[0];
    assert_eq!(cafe["merchant_name"], "Café Olimpico");
    assert_eq!(cafe["original_statement"], "CAFE OLIMPICO MTL");
    assert_eq!(cafe["source"], "import");
    assert_eq!(cafe["reviewed"], true);
    let iga = &transactions[1];
    assert_eq!(iga["merchant_id"], iga_id);
    assert_eq!(iga["amount"], "142.37");
    assert_eq!(iga["source"], "import");
    assert_eq!(iga["tags"][0]["id"], tag_id);
    assert_eq!(transactions[2]["source"], "manual");
}

#[sqlx::test]
async fn import_transactions_reuses_a_merchant_with_the_same_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let existing_id =
        create_via_api(&app, &cookie, category_id, "2026-09-01", "BIXI", "5.00").await;
    let bixi_id = transaction_merchant_id(&app, &cookie, existing_id).await;

    let line = serde_json::json!({
        "date": "2026-09-22",
        "original_statement": "BIXI MONTREAL",
        "amount": "15.00",
        "new_merchant_name": "bixi",
        "category_id": category_id,
        "account_id": account_id
    });
    let resp = import_via_api(&app, &cookie, serde_json::json!([line.clone(), line])).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["created_merchant_count"], 0);
    let transactions = list_all(&app, &cookie).await;
    assert!(transactions.iter().all(|t| t["merchant_id"] == bixi_id));
}

#[sqlx::test]
async fn import_transactions_creates_a_new_account_once(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let line = |statement: &str| {
        serde_json::json!({
            "date": "2026-09-02",
            "original_statement": statement,
            "amount": "10.00",
            "new_merchant_name": statement,
            "category_id": category_id,
            "new_account": {
                "institution_name": "RBC",
                "name": "Mastercard",
                "type": "credit_card",
                "last_four": "9999"
            }
        })
    };
    let resp = import_via_api(&app, &cookie, serde_json::json!([line("A"), line("B")])).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["created_account_count"], 1);

    let req = test::TestRequest::get()
        .uri("/accounts")
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let accounts: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let accounts = accounts.as_array().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0]["name"], "Mastercard");
    assert_eq!(accounts[0]["institution_name"], "RBC");
    assert_eq!(accounts[0]["last_four"], "9999");
    let transactions = list_all(&app, &cookie).await;
    assert!(transactions
        .iter()
        .all(|t| t["account_id"] == accounts[0]["id"]));

    // A second import naming the same account reuses it.
    let resp = import_via_api(&app, &cookie, serde_json::json!([line("C")])).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["created_account_count"], 0);
}

#[sqlx::test]
async fn import_transactions_reuses_an_account_saved_without_last_digits(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let req = test::TestRequest::post()
        .uri("/institutions")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({ "name": "Desjardins" }))
        .to_request();
    let institution: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "institution_id": institution["id"],
            "name": "Visa",
            "type": "credit_card",
        }))
        .to_request();
    let visa: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    // The statement names the bank its own way and prints last digits the account doesn't have.
    let resp = import_via_api(
        &app,
        &cookie,
        serde_json::json!([{
            "date": "2026-09-02",
            "original_statement": "IGA",
            "amount": "1.00",
            "new_merchant_name": "IGA",
            "category_id": category_id,
            "new_account": {
                "institution_name": "Caisse Desjardins du Plateau",
                "name": "visa",
                "type": "credit_card",
                "last_four": "1234"
            }
        }]),
    )
    .await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["created_account_count"], 0);
    let transactions = list_all(&app, &cookie).await;
    assert_eq!(transactions[0]["account_id"], visa["id"]);
    let req = test::TestRequest::get()
        .uri("/institutions")
        .insert_header(("Cookie", cookie))
        .to_request();
    let institutions: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    assert!(!institutions
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["name"] == "Caisse Desjardins du Plateau"));
}

#[sqlx::test]
async fn import_transactions_needs_exactly_one_valid_account_per_line(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let new_account = |last_four: &str| {
        serde_json::json!({
            "institution_name": "RBC",
            "name": "Mastercard",
            "type": "credit_card",
            "last_four": last_four
        })
    };

    for account in [
        serde_json::json!({}),
        serde_json::json!({ "account_id": account_id, "new_account": new_account("9999") }),
        // Not what `accounts.last_four` accepts.
        serde_json::json!({ "new_account": new_account("99-999") }),
    ] {
        let mut line = serde_json::json!({
            "date": "2026-09-02",
            "original_statement": "IGA",
            "amount": "1.00",
            "new_merchant_name": "IGA",
            "category_id": category_id
        });
        for (key, value) in account.as_object().unwrap() {
            line[key] = value.clone();
        }
        let resp = import_via_api(&app, &cookie, serde_json::json!([line])).await;
        assert_eq!(resp.status(), 400);
    }
}

#[sqlx::test]
async fn import_transactions_is_all_or_nothing(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;

    let resp = import_via_api(
        &app,
        &cookie,
        serde_json::json!([
            {
                "date": "2026-09-02",
                "original_statement": "IGA",
                "amount": "1.00",
                "new_merchant_name": "IGA",
                "category_id": category_id,
                "account_id": account_id
            },
            {
                "date": "2026-09-03",
                "original_statement": "STM",
                "amount": "2.00",
                "new_merchant_name": "STM",
                "category_id": 999999,
                "account_id": account_id
            }
        ]),
    )
    .await;

    assert_eq!(resp.status(), 400);
    assert!(list_all(&app, &cookie).await.is_empty());
}

#[sqlx::test]
async fn import_transactions_rejects_another_households_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let other_cookie = sign_in_with_household(&app, "other@example.com").await;
    let other_category_id = create_other_category(&app, &other_cookie).await;
    let other_transaction_id = create_via_api(
        &app,
        &other_cookie,
        other_category_id,
        "2026-09-01",
        "SECRET SHOP",
        "1.00",
    )
    .await;
    let other_merchant_id =
        transaction_merchant_id(&app, &other_cookie, other_transaction_id).await;

    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;

    let resp = import_via_api(
        &app,
        &cookie,
        serde_json::json!([{
            "date": "2026-09-02",
            "original_statement": "SECRET SHOP",
            "amount": "1.00",
            "merchant_id": other_merchant_id,
            "category_id": category_id,
            "account_id": account_id
        }]),
    )
    .await;

    assert_eq!(resp.status(), 400);
    assert!(list_all(&app, &cookie).await.is_empty());
}

#[sqlx::test]
async fn import_transactions_needs_exactly_one_merchant_per_line(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let account_id = crate::shared::test_support::test_account_id(&app, &cookie).await;
    let existing_id = create_via_api(&app, &cookie, category_id, "2026-09-01", "IGA", "1.00").await;
    let iga_id = transaction_merchant_id(&app, &cookie, existing_id).await;

    let line = |merchant: serde_json::Value| {
        let mut line = serde_json::json!({
            "date": "2026-09-02",
            "original_statement": "IGA",
            "amount": "1.00",
            "category_id": category_id,
            "account_id": account_id
        });
        for (key, value) in merchant.as_object().unwrap() {
            line[key] = value.clone();
        }
        line
    };

    for merchant in [
        serde_json::json!({}),
        serde_json::json!({ "new_merchant_name": "   " }),
        serde_json::json!({ "merchant_id": iga_id, "new_merchant_name": "IGA" }),
    ] {
        let resp = import_via_api(&app, &cookie, serde_json::json!([line(merchant)])).await;
        assert_eq!(resp.status(), 400);
    }
}

#[sqlx::test]
async fn import_transactions_rejects_an_empty_list(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let resp = import_via_api(&app, &cookie, serde_json::json!([])).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn import_transactions_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::post()
        .uri("/transactions/import")
        .set_json(serde_json::json!({ "transactions": [] }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}
