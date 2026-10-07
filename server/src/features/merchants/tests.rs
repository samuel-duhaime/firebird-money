//! Integration tests for the merchants HTTP API.
//!
//! Each `#[sqlx::test]` gets its own throwaway Postgres database (migrated from `migrations/`,
//! dropped afterwards), so these never touch real dev data. Unlike `households::repository::create`,
//! which seeds a fresh household's categories/tags automatically, the common merchants are only
//! seeded by `repository::seed_defaults` — called once at real server startup, never implicitly by a
//! migration or by creating a household — so a test that needs them present calls it explicitly.

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App};
use sqlx::PgPool;

use super::handlers::configure;
use super::repository;
use crate::features::auth;
use crate::shared::config::{AuthConfig, SESSION_COOKIE_NAME};
use crate::shared::l10n::L10n;

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

/// Creates a custom merchant through `POST /merchants` and returns its id.
async fn create_via_api<S, B>(app: &S, cookie: &str, name: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/merchants")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": name }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created merchant, got {body}"))
}

async fn list_via_api<S, B>(app: &S, cookie: &str) -> Vec<serde_json::Value>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/merchants")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body.as_array().unwrap().clone()
}

async fn list_via_api_ordered<S, B>(app: &S, cookie: &str, order: &str) -> Vec<serde_json::Value>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri(&format!("/merchants?order={order}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body.as_array().unwrap().clone()
}

/// The `household.household_id` of the signed-in caller, per `GET /auth/me` — same helper
/// `transactions::tests` uses.
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

/// Inserts a category group + category directly via SQL (categories/category_groups aren't wired
/// into this module's `app_with`, and these tests only need a valid `category_id` to attach
/// transactions to, not the categories API itself). Returns the category's id.
async fn insert_category(pool: &PgPool, household_id: i32, name: &str) -> i32 {
    let (group_id,): (i32,) = sqlx::query_as(
        "INSERT INTO category_groups (household_id, name_en, name_fr, type, sort_order)
         VALUES ($1, $2, $2, 'expense', 0) RETURNING id",
    )
    .bind(household_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap();

    let (category_id,): (i32,) = sqlx::query_as(
        "INSERT INTO categories (household_id, group_id, name_en, name_fr, sort_order)
         VALUES ($1, $2, $3, $3, 0) RETURNING id",
    )
    .bind(household_id)
    .bind(group_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap();

    category_id
}

/// Inserts a transaction directly via SQL, for tests that only need transaction history to exist
/// (e.g. `recommended_category_id`), not the transactions API itself.
#[allow(clippy::too_many_arguments)]
async fn insert_transaction(
    pool: &PgPool,
    household_id: i32,
    household_member_id: i64,
    merchant_id: i64,
    category_id: i32,
    date: &str,
) {
    sqlx::query(
        "INSERT INTO transactions
            (household_id, household_member_id, date, original_statement, merchant_id, amount,
             category_id, account, reviewed)
         VALUES ($1, $2, $3::date, 'test', $4, 10.00, $5, 'Test Account', true)",
    )
    .bind(household_id)
    .bind(household_member_id)
    .bind(date)
    .bind(merchant_id)
    .bind(category_id)
    .execute(pool)
    .await
    .unwrap();
}

// --- POST /merchants ---

#[sqlx::test]
async fn create_merchant_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/merchants")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Corner Store" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Corner Store");
    assert!(body["household_id"].is_i64());
    assert_eq!(body["recommended_category_id"], serde_json::Value::Null);
    assert_eq!(body["transaction_count"].as_i64().unwrap(), 0);
}

#[sqlx::test]
async fn create_merchant_rejects_a_duplicate_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    create_via_api(&app, &cookie, "Corner Store").await;

    let req = test::TestRequest::post()
        .uri("/merchants")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Corner Store" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

// --- GET /merchants ---

#[sqlx::test]
async fn list_merchants_includes_common_merchants_once_seeded(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let merchants = list_via_api(&app, &cookie).await;

    let amazon = merchants
        .iter()
        .find(|m| m["name"] == "Amazon")
        .expect("Amazon should be seeded as a common merchant");
    assert_eq!(amazon["household_id"], serde_json::Value::Null);
}

#[sqlx::test]
async fn list_merchants_does_not_leak_another_households_custom_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    create_via_api(&app, &cookie_a, "A's Shop").await;

    let merchants_b = list_via_api(&app, &cookie_b).await;

    assert!(!merchants_b.iter().any(|m| m["name"] == "A's Shop"));
}

#[sqlx::test]
async fn list_merchants_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get().uri("/merchants").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn get_merchant_404s_for_an_unknown_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/merchants/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /merchants/{id} ---

#[sqlx::test]
async fn update_merchant_renames_a_custom_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Old Name").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/merchants/{id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "New Name" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "New Name");
}

#[sqlx::test]
async fn update_merchant_404s_on_a_common_merchant(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let merchants = list_via_api(&app, &cookie).await;
    let amazon_id = merchants
        .iter()
        .find(|m| m["name"] == "Amazon")
        .expect("Amazon should be seeded")["id"]
        .as_i64()
        .unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/merchants/{amazon_id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Hacked" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /merchants/{id} ---

#[sqlx::test]
async fn delete_merchant_deletes_a_custom_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie, "Short-Lived Shop").await;

    let req = test::TestRequest::delete()
        .uri(&format!("/merchants/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 204);
}

#[sqlx::test]
async fn delete_merchant_fails_while_used_by_a_transaction(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let household_member_id = own_household_member_id(&app, &cookie).await;
    let merchant_id = create_via_api(&app, &cookie, "In-Use Shop").await;
    let category_id = insert_category(&pool, household_id, "Category Test").await;
    insert_transaction(
        &pool,
        household_id,
        household_member_id,
        merchant_id,
        category_id,
        "2026-01-01",
    )
    .await;

    let req = test::TestRequest::delete()
        .uri(&format!("/merchants/{merchant_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);

    // The merchant and its transaction are both still there — the delete was refused, not partial.
    let merchant = repository::get(&pool, household_id, merchant_id as i32)
        .await
        .unwrap();
    assert!(merchant.is_some());
}

#[sqlx::test]
async fn delete_merchant_404s_on_a_common_merchant(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let merchants = list_via_api(&app, &cookie).await;
    let amazon_id = merchants
        .iter()
        .find(|m| m["name"] == "Amazon")
        .expect("Amazon should be seeded")["id"]
        .as_i64()
        .unwrap();

    let req = test::TestRequest::delete()
        .uri(&format!("/merchants/{amazon_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- seed_defaults ---

#[sqlx::test]
async fn seed_defaults_is_idempotent(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    repository::seed_defaults(&pool).await.unwrap();

    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM merchants WHERE name = 'Amazon'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn seed_defaults_populates_aliases(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();

    let (aliases,): (Vec<String>,) =
        sqlx::query_as("SELECT aliases FROM merchants WHERE name = $1")
            .bind("Hudson's Bay")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(aliases.contains(&"Hudsons Bay".to_string()));
    assert!(aliases.contains(&"Hudson Bay".to_string()));
    assert!(aliases.contains(&"The Bay".to_string()));
}

// --- resolve_or_create / matching (the "SPOTIFY P0A1B2" -> Spotify, alias, and no-match cases) ---

#[sqlx::test]
async fn resolve_or_create_matches_an_existing_merchant_by_name(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let spotify_id = list_via_api(&app, &cookie)
        .await
        .into_iter()
        .find(|m| m["name"] == "Spotify")
        .unwrap()["id"]
        .as_i64()
        .unwrap() as i32;

    let mut conn = pool.acquire().await.unwrap();
    let matched = repository::resolve_or_create(&mut conn, household_id, "SPOTIFY P0A1B2")
        .await
        .unwrap();

    assert_eq!(matched, spotify_id);
}

#[sqlx::test]
async fn resolve_or_create_matches_via_an_alias_not_the_real_name(pool: PgPool) {
    // "McDonald's" is the real name; most bank statements drop the apostrophe entirely, which is
    // exactly what the "McDonalds" alias (see defaults::DEFAULT_MERCHANTS) is for.
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let mcdonalds_id = list_via_api(&app, &cookie)
        .await
        .into_iter()
        .find(|m| m["name"] == "McDonald's")
        .unwrap()["id"]
        .as_i64()
        .unwrap() as i32;

    let mut conn = pool.acquire().await.unwrap();
    let matched = repository::resolve_or_create(&mut conn, household_id, "MCDONALDS #04521")
        .await
        .unwrap();

    assert_eq!(matched, mcdonalds_id);
}

#[sqlx::test]
async fn resolve_or_create_matches_an_existing_custom_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let local_shop_id = create_via_api(&app, &cookie, "Local Shop").await as i32;

    let mut conn = pool.acquire().await.unwrap();
    let matched = repository::resolve_or_create(&mut conn, household_id, "LOCAL SHOP RECEIPT 123")
        .await
        .unwrap();

    assert_eq!(matched, local_shop_id);
}

#[sqlx::test]
async fn resolve_or_create_creates_a_new_merchant_when_nothing_matches(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;

    let mut conn = pool.acquire().await.unwrap();
    let created_id = repository::resolve_or_create(&mut conn, household_id, "NotImportantMerchantName")
        .await
        .unwrap();

    let merchant = repository::get(&pool, household_id, created_id)
        .await
        .unwrap()
        .expect("the new merchant should exist");
    assert_eq!(merchant.name, "NotImportantMerchantName");
    assert_eq!(merchant.household_id, Some(household_id));
}

#[sqlx::test]
async fn resolve_or_create_does_not_create_a_duplicate_on_a_second_call(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;

    let mut conn = pool.acquire().await.unwrap();
    let first = repository::resolve_or_create(&mut conn, household_id, "Repeat Merchant")
        .await
        .unwrap();
    let second = repository::resolve_or_create(&mut conn, household_id, "Repeat Merchant")
        .await
        .unwrap();

    assert_eq!(first, second);
}

// --- is_visible_to_household ---

#[sqlx::test]
async fn is_visible_to_household_is_true_for_a_common_merchant(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let amazon_id = list_via_api(&app, &cookie)
        .await
        .into_iter()
        .find(|m| m["name"] == "Amazon")
        .unwrap()["id"]
        .as_i64()
        .unwrap() as i32;

    assert!(
        repository::is_visible_to_household(&pool, household_id, amazon_id)
            .await
            .unwrap()
    );
}

#[sqlx::test]
async fn is_visible_to_household_is_false_for_another_households_custom_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let household_b = own_household_id(&app, &cookie_b).await;
    let merchant_a_id = create_via_api(&app, &cookie_a, "A's Shop").await as i32;

    assert!(
        !repository::is_visible_to_household(&pool, household_b, merchant_a_id)
            .await
            .unwrap()
    );
}

// --- cross-household isolation on GET ---

#[sqlx::test]
async fn get_merchant_404s_for_another_households_custom_merchant(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let merchant_a_id = create_via_api(&app, &cookie_a, "A's Shop").await;

    let req = test::TestRequest::get()
        .uri(&format!("/merchants/{merchant_a_id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- sorting ---

#[sqlx::test]
async fn list_merchants_sorts_alphabetically_when_requested(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    create_via_api(&app, &cookie, "Zebra Shop").await;
    create_via_api(&app, &cookie, "Alpha Shop").await;

    let merchants = list_via_api_ordered(&app, &cookie, "alphabetical").await;
    let names: Vec<&str> = merchants.iter().map(|m| m["name"].as_str().unwrap()).collect();

    let alpha_index = names.iter().position(|n| *n == "Alpha Shop").unwrap();
    let zebra_index = names.iter().position(|n| *n == "Zebra Shop").unwrap();
    assert!(alpha_index < zebra_index);
}

// --- recommended_category_id ---

#[sqlx::test]
async fn recommended_category_id_is_null_with_no_transactions(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/merchants")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Fresh Merchant" }))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["recommended_category_id"], serde_json::Value::Null);
}

#[sqlx::test]
async fn recommended_category_id_is_the_most_used_category(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let household_member_id = own_household_member_id(&app, &cookie).await;
    let merchant_id = create_via_api(&app, &cookie, "Amazon Test").await;
    let shopping = insert_category(&pool, household_id, "Shopping Test").await;
    let electronics = insert_category(&pool, household_id, "Electronics Test").await;

    // Shopping used twice, Electronics once — Shopping should win.
    insert_transaction(
        &pool,
        household_id,
        household_member_id,
        merchant_id,
        shopping,
        "2026-01-01",
    )
    .await;
    insert_transaction(
        &pool,
        household_id,
        household_member_id,
        merchant_id,
        shopping,
        "2026-01-02",
    )
    .await;
    insert_transaction(
        &pool,
        household_id,
        household_member_id,
        merchant_id,
        electronics,
        "2026-01-03",
    )
    .await;

    let merchant = repository::get(&pool, household_id, merchant_id as i32)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(merchant.recommended_category_id, Some(shopping));
    assert_eq!(merchant.transaction_count, 3);
}

#[sqlx::test]
async fn recommended_category_id_breaks_a_tie_with_the_most_recent_transaction(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let household_id = own_household_id(&app, &cookie).await;
    let household_member_id = own_household_member_id(&app, &cookie).await;
    let merchant_id = create_via_api(&app, &cookie, "Amazon Test").await;
    let shopping = insert_category(&pool, household_id, "Shopping Test").await;
    let electronics = insert_category(&pool, household_id, "Electronics Test").await;

    // One transaction each, Electronics more recent — Electronics should win the tie.
    insert_transaction(
        &pool,
        household_id,
        household_member_id,
        merchant_id,
        shopping,
        "2026-01-01",
    )
    .await;
    insert_transaction(
        &pool,
        household_id,
        household_member_id,
        merchant_id,
        electronics,
        "2026-01-15",
    )
    .await;

    let merchant = repository::get(&pool, household_id, merchant_id as i32)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(merchant.recommended_category_id, Some(electronics));
}
