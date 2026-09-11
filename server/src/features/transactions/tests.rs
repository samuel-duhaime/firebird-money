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
use super::jobs::JobStore;
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
    app_with_jobs(pool, web::Data::new(JobStore::default()))
}

/// Like `app_with`, but takes an externally-owned `JobStore` so a test can seed a job directly
/// (via `JobStore::create`) without going through `POST /transactions/import`, which would spawn
/// a real `claude` subprocess.
fn app_with_jobs(
    pool: PgPool,
    job_store: web::Data<JobStore>,
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
        .app_data(job_store)
        .configure(configure)
        .configure(auth::configure)
        .configure(crate::features::category_groups::configure)
        .configure(crate::features::categories::configure)
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

/// The `household.id` (i.e. `household_member_id`) of the signed-in caller, per `GET /auth/me`.
async fn own_household_member_id<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body["household"]["id"].as_i64().unwrap()
}

/// The `household.household_id` of the signed-in caller, per `GET /auth/me`.
async fn own_household_id<S, B>(app: &S, cookie: &str) -> i32
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(app, req).await;
    body["household"]["household_id"].as_i64().unwrap() as i32
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

/// Creates a transaction through `POST /transactions` and returns its id, for tests that only
/// need an existing row to act on.
async fn create_via_api<S, B>(
    app: &S,
    cookie: &str,
    category_id: i64,
    date: &str,
    merchant: &str,
    amount: &str,
) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": date,
            "merchant": merchant,
            "amount": amount,
            "category_id": category_id,
            "account": "User 1",
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created transaction, got {body}"))
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
async fn list_transactions_filters_by_merchant(pool: PgPool) {
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
        .uri("/transactions?merchant=iga")
        .insert_header(("Cookie", cookie))
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
    assert_eq!(rows[0]["merchant"], "STARBUCKS");
}

#[sqlx::test]
async fn list_transactions_filters_by_date_and_merchant_combined(pool: PgPool) {
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
    create_via_api(&app, &cookie, category_id, "2024-01-15", "IGA", "56.78").await;
    create_via_api(&app, &cookie, category_id, "2024-01-16", "IGA", "78.90").await;

    let req = test::TestRequest::get()
        .uri("/transactions?date=2024-01-15&merchant=iga")
        .insert_header(("Cookie", cookie))
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
    assert_eq!(rows[0]["merchant"], "IGA");
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
    assert_eq!(rows[0]["merchant"], "STARBUCKS");
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
    assert_eq!(rows[0]["merchant"], "STARBUCKS");
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
    assert_eq!(rows[0]["merchant"], "SCHOOL SUPPLIES");
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
    assert_eq!(rows[0]["merchant"], "STARBUCKS");
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
    assert_eq!(rows[0]["merchant"], "50% OFF STORE");
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
    assert_eq!(rows[0]["merchant"], "ITEM_CODE 123");
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
    assert_eq!(rows[0]["merchant"], r"PATH\TO STORE");
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
        .uri("/transactions?merchant=nonexistent")
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
    assert_eq!(lines.next().unwrap(), "Date,Merchant,Category,Amount");
    assert_eq!(lines.next().unwrap(), "2024-01-16,IGA,Other,56.78");
    assert_eq!(lines.next().unwrap(), "2024-01-15,STARBUCKS,Other,12.34");
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
    assert_eq!(rows[0], "2024-01-15,'=SUM(A1:A2),Other,12.34");
    assert_eq!(rows[1], "2024-01-16,'+1234567890,Other,20.00");
    assert_eq!(rows[2], "2024-01-17,'-2+3,Other,30.00");
    assert_eq!(rows[3], "2024-01-18,'@SUM(A1),Other,40.00");
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
        "2024-01-15,STARBUCKS,'=cmd,12.34"
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
    assert_eq!(lines.next().unwrap(), "Date,Merchant,Category,Amount");
    assert_eq!(lines.next().unwrap(), "2024-01-15,STARBUCKS,Other,12.34");
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
    assert_eq!(lines.next().unwrap(), "2024-01-15,STARBUCKS,Other,12.34");
    assert_eq!(lines.next().unwrap(), "2024-01-16,IGA,Other,56.78");
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
    assert_eq!(body["merchant"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["category_name_en"], "Other");
    assert_eq!(body["category_name_fr"], "Autre");
    assert_eq!(body["category_type"], "expense");
    assert!(body["household_id"].is_i64());
    assert!(body["household_member_id"].is_i64());
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

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
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
async fn create_transaction_attributes_it_to_the_callers_membership(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;
    let expected_member_id = own_household_member_id(&app, &cookie).await;

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account": "User 1",
        }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(
        body["household_member_id"].as_i64().unwrap(),
        expected_member_id
    );
}

#[sqlx::test]
async fn create_transaction_requires_a_session(pool: PgPool) {
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
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": 1,
            "account": "User 1",
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
        .set_json(serde_json::json!({ "merchant": "STARBUCKS" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_transaction_persists_all_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = education_category_id(&app, &cookie).await;

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account": "User 2",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["date"], "2024-01-15");
    assert_eq!(body["merchant"], "STARBUCKS");
    assert_eq!(body["amount"], "12.34");
    assert_eq!(body["category_id"].as_i64().unwrap(), category_id);
    assert_eq!(body["category_name_en"], "Education");
    assert_eq!(body["category_name_fr"], "Éducation");
    assert_eq!(body["category_type"], "expense");
    assert_eq!(body["account"], "User 2");
}

#[sqlx::test]
async fn create_transaction_defaults_reviewed_to_true(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let category_id = create_other_category(&app, &cookie).await;

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account": "User 1",
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

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
            "account": "User 1",
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

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
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
async fn create_transaction_rejects_another_households_category(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let category_a = create_other_category(&app, &cookie_a).await;

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({
            "date": "2024-01-15",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_a,
            "account": "User 1",
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

    let req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "date": "not-a-date",
            "merchant": "STARBUCKS",
            "amount": "12.34",
            "category_id": category_id,
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
    assert_eq!(body["merchant"], "STARBUCKS");
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
    assert_eq!(body["merchant"], "STARBUCKS");
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

// --- POST /transactions/import ---
//
// Only the size-limit rejection is tested here: it's rejected during multipart extraction, before
// `import_transactions` ever runs, so — unlike a real successful upload — it never spawns a
// `claude` subprocess.

#[sqlx::test]
async fn import_transactions_rejects_a_file_over_the_size_limit(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let oversized = vec![b'a'; 11 * 1024 * 1024]; // over the 10 MB field limit
    let (body, headers) = actix_multipart::test::create_form_data_payload_and_headers(
        "file",
        Some("statement.csv".to_string()),
        None,
        web::Bytes::from(oversized),
    );

    let mut req = test::TestRequest::post().uri("/transactions/import");
    for (name, value) in headers.iter() {
        req = req.insert_header((name.clone(), value.clone()));
    }
    let resp = test::call_service(&app, req.set_payload(body).to_request()).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    // Pins down the specific message, not just "some JSON error" — the field-level size limit
    // surfaces as `MultipartError::Payload(PayloadError::Overflow)`, easy to mis-map to the wrong
    // Fluent key (as a first pass here did) since it isn't `MultipartError::Field`.
    assert!(body["error"].as_str().unwrap().contains("too large"));
}

// --- /transactions/import/jobs/{id} ---
//
// These test the HTTP wiring for the in-memory JobStore directly (seeding a job via
// `JobStore::create`), never through `POST /transactions/import` — that endpoint spawns a real
// `claude` subprocess, which has no place in an automated test suite.

#[sqlx::test]
async fn get_import_job_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/transactions/import/jobs/{}",
            uuid::Uuid::new_v4()
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn get_import_job_returns_404_for_unknown_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!(
            "/transactions/import/jobs/{}",
            uuid::Uuid::new_v4()
        ))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn get_import_job_returns_a_freshly_created_job_as_pending(pool: PgPool) {
    let job_store = web::Data::new(JobStore::default());
    let app = test::init_service(app_with_jobs(pool, job_store.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let job = job_store.create("statement.csv".to_string(), household_id);

    let req = test::TestRequest::get()
        .uri(&format!("/transactions/import/jobs/{}", job.id))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "pending");
    assert_eq!(body["file_name"], "statement.csv");
}

#[sqlx::test]
async fn report_import_job_requires_a_session(pool: PgPool) {
    let job_store = web::Data::new(JobStore::default());
    let job = job_store.create("statement.csv".to_string(), 1);
    let app = test::init_service(app_with_jobs(pool, job_store)).await;

    let patch_req = test::TestRequest::patch()
        .uri(&format!("/transactions/import/jobs/{}", job.id))
        .set_json(serde_json::json!({ "status": "succeeded" }))
        .to_request();
    let patch_resp = test::call_service(&app, patch_req).await;

    assert_eq!(patch_resp.status(), 401);
}

#[sqlx::test]
async fn report_import_job_updates_status_and_get_reflects_it(pool: PgPool) {
    let job_store = web::Data::new(JobStore::default());
    let app = test::init_service(app_with_jobs(pool, job_store.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let job = job_store.create("statement.csv".to_string(), household_id);

    let patch_req = test::TestRequest::patch()
        .uri(&format!("/transactions/import/jobs/{}", job.id))
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "status": "succeeded",
            "created_count": 3,
            "failed_count": 0,
            "skipped_count": 1,
        }))
        .to_request();
    let patch_resp = test::call_service(&app, patch_req).await;
    assert_eq!(patch_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/transactions/import/jobs/{}", job.id))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;

    assert_eq!(get_resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(get_resp).await;
    assert_eq!(body["status"], "succeeded");
    assert_eq!(body["created_count"], 3);
    assert_eq!(body["skipped_count"], 1);
}

#[sqlx::test]
async fn get_import_job_does_not_leak_another_households_job(pool: PgPool) {
    let job_store = web::Data::new(JobStore::default());
    let app = test::init_service(app_with_jobs(pool, job_store.clone())).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let household_a = own_household_id(&app, &cookie_a).await;
    let job = job_store.create("statement.csv".to_string(), household_a);
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!("/transactions/import/jobs/{}", job.id))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn report_import_job_rejects_another_households_job(pool: PgPool) {
    let job_store = web::Data::new(JobStore::default());
    let app = test::init_service(app_with_jobs(pool, job_store.clone())).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let household_a = own_household_id(&app, &cookie_a).await;
    let job = job_store.create("statement.csv".to_string(), household_a);
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;

    let patch_req = test::TestRequest::patch()
        .uri(&format!("/transactions/import/jobs/{}", job.id))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "status": "succeeded" }))
        .to_request();
    let patch_resp = test::call_service(&app, patch_req).await;

    assert_eq!(patch_resp.status(), 404);
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
