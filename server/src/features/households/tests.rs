//! Integration tests for the households HTTP API.
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

/// Signs in through `POST /auth/request-login` and returns the session cookie to replay on later
/// requests (the test client doesn't keep a cookie jar).
async fn sign_in<S, B>(app: &S, email: &str) -> String
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

    format!("{}={}", cookie.name(), cookie.value())
}

/// Creates a household through `POST /households` and returns its id, for tests that only need an
/// existing row to act on.
async fn create_via_api<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/households")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created household, got {body}"))
}

// --- POST /households ---

#[sqlx::test]
async fn create_household_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/households")
        .insert_header(("Cookie", cookie))
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
    assert!(body["id"].is_i64());
    assert!(body["created_at"].is_string());
    assert_eq!(
        location,
        format!("/households/{}", body["id"].as_i64().unwrap())
    );
}

#[sqlx::test]
async fn create_household_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post().uri("/households").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

// --- GET /households/{id} ---

#[sqlx::test]
async fn get_household_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie).await;

    let req = test::TestRequest::get()
        .uri(&format!("/households/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["id"].as_i64().unwrap(), id);
}

#[sqlx::test]
async fn get_household_omits_join_code(pool: PgPool) {
    // `join_code` is a shared secret; it must not be handed to just anyone who can guess an id.
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie).await;

    let req = test::TestRequest::get()
        .uri(&format!("/households/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert!(body.get("join_code").is_none());
}

#[sqlx::test]
async fn create_household_returns_its_join_code(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/households")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["join_code"].as_str().unwrap().len(), 8);
}

#[sqlx::test]
async fn create_household_seeds_default_category_groups_and_categories(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie).await;

    let group_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM category_groups WHERE household_id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .expect("query seeded category groups");
    assert_eq!(
        group_count.0, 16,
        "every household starts with the same starter groups"
    );

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT c.name_en, g.type
         FROM categories c
         JOIN category_groups g ON g.id = c.group_id
         WHERE c.household_id = $1",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .expect("query seeded categories");

    assert_eq!(
        rows.len(),
        65,
        "every household starts with the same starter categories"
    );
    assert!(rows
        .iter()
        .any(|(name, kind)| name == "Groceries" && kind == "expense"));
    assert!(rows
        .iter()
        .any(|(name, kind)| name == "Paychecks" && kind == "income"));
    assert!(rows
        .iter()
        .any(|(name, kind)| name == "TFSA" && kind == "transfer"));
}

#[sqlx::test]
async fn get_household_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri("/households/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

// --- DELETE /households/{id} ---

#[sqlx::test]
async fn delete_household_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in(&app, "sam@example.com").await;
    let id = create_via_api(&app, &cookie).await;

    // A household gets its starter category groups (and categories inside them) seeded at
    // creation, which would otherwise block this delete (see
    // `delete_household_rejects_when_referenced_by_member`) — clear them directly, categories
    // first per the FK, so this test isolates "deleting an otherwise-unreferenced household
    // succeeds".
    sqlx::query("DELETE FROM categories WHERE household_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("clear seeded categories");
    sqlx::query("DELETE FROM category_groups WHERE household_id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("clear seeded category groups");

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/households/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/households/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_household_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::delete()
        .uri("/households/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_household_rejects_when_referenced_by_member(pool: PgPool) {
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool))
            .app_data(web::Data::new(L10n::new()))
            .app_data(web::Data::new(test_config()))
            .app_data(web::Data::new(reqwest::Client::new()))
            .configure(configure)
            .configure(auth::configure)
            .configure(crate::features::users::configure)
            .configure(crate::features::household_members::configure),
    )
    .await;
    let cookie = sign_in(&app, "sam@example.com").await;
    let household_id = create_via_api(&app, &cookie).await;

    let user_req = test::TestRequest::post()
        .uri("/users")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({ "email": "member@example.com" }))
        .to_request();
    let user_resp = test::call_service(&app, user_req).await;
    let user_body: serde_json::Value = test::read_body_json(user_resp).await;
    let user_id = user_body["id"].as_i64().unwrap();

    let member_req = test::TestRequest::post()
        .uri("/household-members")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "household_id": household_id,
            "user_id": user_id,
            "type": "family_manager",
        }))
        .to_request();
    let member_resp = test::call_service(&app, member_req).await;
    assert_eq!(member_resp.status(), 201);

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/households/{household_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 409);
    let body: serde_json::Value = test::read_body_json(delete_resp).await;
    assert!(body["error"].is_string());
}
