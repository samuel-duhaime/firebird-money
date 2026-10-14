//! Integration tests for the institutions HTTP API. Each `#[sqlx::test]` gets its own throwaway
//! database; the common institutions are only present when a test calls `seed_defaults` itself.

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App};
use sqlx::PgPool;

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
        .configure(auth::configure)
        .configure(crate::features::institutions::configure)
        .configure(crate::features::accounts::configure)
        .configure(crate::features::transactions::configure)
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
        .set_json(serde_json::json!({ "first_name": "Test", "last_name": "User" }))
        .to_request();
    assert_eq!(test::call_service(app, onboard_req).await.status(), 201);

    cookie
}

/// Sends `method` to `uri` as the caller, with an optional JSON body; returns the status and the
/// parsed body (`Null` for an empty one).
async fn call<S, B>(
    app: &S,
    cookie: &str,
    req: test::TestRequest,
    body: Option<serde_json::Value>,
) -> (u16, serde_json::Value)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let mut req = req.insert_header(("Cookie", cookie));
    if let Some(body) = body {
        req = req.set_json(body);
    }
    let resp = test::call_service(app, req.to_request()).await;
    let status = resp.status().as_u16();
    let bytes = test::read_body(resp).await;
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

/// Creates a custom institution and returns its id.
async fn create_institution<S, B>(app: &S, cookie: &str, name: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let (status, body) = call(
        app,
        cookie,
        test::TestRequest::post().uri("/institutions"),
        Some(serde_json::json!({ "name": name })),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    body["id"].as_i64().unwrap()
}

/// Creates an account at `institution_id` and returns its id.
async fn create_account<S, B>(app: &S, cookie: &str, institution_id: i64, name: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let (status, body) = call(
        app,
        cookie,
        test::TestRequest::post().uri("/accounts"),
        Some(serde_json::json!({
            "institution_id": institution_id,
            "name": name,
            "type": "checking",
        })),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    body["id"].as_i64().unwrap()
}

use super::repository;

#[sqlx::test]
async fn list_institutions_includes_common_ones_once_seeded(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let (status, body) = call(
        &app,
        &cookie,
        test::TestRequest::get().uri("/institutions"),
        None,
    )
    .await;

    assert_eq!(status, 200);
    let desjardins = body
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["name"] == "Desjardins")
        .expect("Desjardins should be seeded");
    assert_eq!(desjardins["household_id"], serde_json::Value::Null);
    assert_eq!(desjardins["website_url"], "https://www.desjardins.com");
    assert_eq!(desjardins["account_count"], 0);
}

#[sqlx::test]
async fn seed_defaults_is_idempotent(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    repository::seed_defaults(&pool).await.unwrap();

    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM institutions WHERE name = 'Desjardins'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test]
async fn list_institutions_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get().uri("/institutions").to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 401);
}

#[sqlx::test]
async fn create_institution_trims_and_returns_the_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let (status, body) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/institutions"),
        Some(serde_json::json!({ "name": "  Caisse du coin  ", "website_url": " https://example.com " })),
    )
    .await;

    assert_eq!(status, 201);
    assert_eq!(body["name"], "Caisse du coin");
    assert_eq!(body["website_url"], "https://example.com");
    assert!(body["household_id"].is_i64());
}

#[sqlx::test]
async fn create_institution_rejects_a_blank_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/institutions"),
        Some(serde_json::json!({ "name": "  " })),
    )
    .await;

    assert_eq!(status, 400);
}

#[sqlx::test]
async fn create_institution_rejects_a_non_http_website(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/institutions"),
        Some(serde_json::json!({ "name": "Shady", "website_url": "javascript:alert(1)" })),
    )
    .await;

    assert_eq!(status, 400);
}

#[sqlx::test]
async fn create_institution_rejects_a_duplicate_name(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    create_institution(&app, &cookie, "Caisse du coin").await;

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/institutions"),
        Some(serde_json::json!({ "name": "Caisse du coin" })),
    )
    .await;

    assert_eq!(status, 409);
}

#[sqlx::test]
async fn institutions_do_not_leak_between_households(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let id = create_institution(&app, &cookie_a, "A's Caisse").await;

    let (_, list) = call(
        &app,
        &cookie_b,
        test::TestRequest::get().uri("/institutions"),
        None,
    )
    .await;
    assert!(!list
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["name"] == "A's Caisse"));

    let (status, _) = call(
        &app,
        &cookie_b,
        test::TestRequest::get().uri(&format!("/institutions/{id}")),
        None,
    )
    .await;
    assert_eq!(status, 404);
}

#[sqlx::test]
async fn update_institution_renames_and_clears_the_website(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let (_, created) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/institutions"),
        Some(serde_json::json!({ "name": "Old", "website_url": "https://example.com" })),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, body) = call(
        &app,
        &cookie,
        test::TestRequest::patch().uri(&format!("/institutions/{id}")),
        Some(serde_json::json!({ "name": "New", "website_url": "" })),
    )
    .await;

    assert_eq!(status, 200);
    assert_eq!(body["name"], "New");
    assert_eq!(body["website_url"], serde_json::Value::Null);
}

#[sqlx::test]
async fn common_institutions_cannot_be_renamed_or_deleted(pool: PgPool) {
    repository::seed_defaults(&pool).await.unwrap();
    let (id,): (i32,) = sqlx::query_as("SELECT id FROM institutions WHERE name = 'Desjardins'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::patch().uri(&format!("/institutions/{id}")),
        Some(serde_json::json!({ "name": "Mine now" })),
    )
    .await;
    assert_eq!(status, 404);

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::delete().uri(&format!("/institutions/{id}")),
        None,
    )
    .await;
    assert_eq!(status, 404);
}

#[sqlx::test]
async fn delete_institution_is_blocked_while_it_has_accounts(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let id = create_institution(&app, &cookie, "Caisse du coin").await;
    let account_id = create_account(&app, &cookie, id, "Chèques").await;

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::delete().uri(&format!("/institutions/{id}")),
        None,
    )
    .await;
    assert_eq!(status, 409);

    let (_, institution) = call(
        &app,
        &cookie,
        test::TestRequest::get().uri(&format!("/institutions/{id}")),
        None,
    )
    .await;
    assert_eq!(institution["account_count"], 1);

    call(
        &app,
        &cookie,
        test::TestRequest::delete().uri(&format!("/accounts/{account_id}")),
        None,
    )
    .await;
    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::delete().uri(&format!("/institutions/{id}")),
        None,
    )
    .await;
    assert_eq!(status, 204);
}
