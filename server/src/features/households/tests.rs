//! Integration tests for the households HTTP API. `GET`/`DELETE /households/{id}` are scoped to
//! the caller's own household — see `handlers.rs`.
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

/// Signs in and completes onboarding (creating a fresh household, making the caller its
/// `family_manager`), returning the session cookie and the new household's id.
async fn sign_in_with_household<S, B>(app: &S, email: &str) -> (String, i64)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let cookie = sign_in(app, email).await;

    let onboard_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({ "first_name": "Test", "last_name": "User" }))
        .to_request();
    let session: serde_json::Value = test::call_and_read_body_json(app, onboard_req).await;
    let household_id = session["household"]["household_id"].as_i64().unwrap();

    (cookie, household_id)
}

/// Creates a household through `POST /households` and returns its id — an orphan, unowned by
/// anyone (unlike `sign_in_with_household`'s), useful only for tests that don't touch
/// `GET`/`DELETE /households/{id}` (both scoped to the caller's own household).
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
    let (cookie, id) = sign_in_with_household(&app, "sam@example.com").await;

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
    let (cookie, id) = sign_in_with_household(&app, "sam@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!("/households/{id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert!(body.get("join_code").is_none());
}

#[sqlx::test]
async fn get_household_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (_cookie_a, household_a) = sign_in_with_household(&app, "a@example.com").await;
    let (cookie_b, _household_b) = sign_in_with_household(&app, "b@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!("/households/{household_a}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
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

/// How many rows of `table` belong to `household_id`.
async fn count_in(pool: &PgPool, table: &str, household_id: i64) -> i64 {
    sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM {table} WHERE household_id = $1"
    ))
    .bind(household_id)
    .fetch_one(pool)
    .await
    .expect("count rows")
}

/// Has `email` join `manager_cookie`'s household with its code, returning their cookie.
async fn join<S, B>(app: &S, manager_cookie: &str, email: &str) -> String
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let me_req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", manager_cookie))
        .to_request();
    let me: serde_json::Value = test::call_and_read_body_json(app, me_req).await;

    let cookie = sign_in(app, email).await;
    let join_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "first_name": "Test",
            "join_code": me["household"]["join_code"],
        }))
        .to_request();
    assert_eq!(test::call_service(app, join_req).await.status(), 201);
    cookie
}

#[sqlx::test]
async fn delete_household_removes_it_and_everything_in_it(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let (cookie, id) = sign_in_with_household(&app, "sam@example.com").await;
    assert!(count_in(&pool, "categories", id).await > 0);

    let req = test::TestRequest::delete()
        .uri(&format!("/households/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 204);

    for table in ["categories", "category_groups", "tags", "users"] {
        assert_eq!(count_in(&pool, table, id).await, 0, "{table} left behind");
    }
    let me_req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    let me: serde_json::Value = test::call_and_read_body_json(&app, me_req).await;
    assert!(me["household"].is_null());
    assert!(me["pending_onboarding_steps"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("household")));
}

#[sqlx::test]
async fn delete_household_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let (_, household_a) = sign_in_with_household(&app, "a@example.com").await;
    let (cookie_b, _) = sign_in_with_household(&app, "b@example.com").await;

    let req = test::TestRequest::delete()
        .uri(&format!("/households/{household_a}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 404);
    assert!(count_in(&pool, "categories", household_a).await > 0);
}

#[sqlx::test]
async fn delete_household_rejects_while_others_belong_to_it(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let (manager_cookie, household_id) = sign_in_with_household(&app, "sam@example.com").await;
    join(&app, &manager_cookie, "member@example.com").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/households/{household_id}"))
        .insert_header(("Cookie", manager_cookie))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 409);
    let body: serde_json::Value = test::read_body_json(delete_resp).await;
    assert!(body["error"].is_string());
    assert_eq!(count_in(&pool, "users", household_id).await, 2);
}

#[sqlx::test]
async fn delete_household_requires_a_family_manager(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let (manager_cookie, household_id) = sign_in_with_household(&app, "sam@example.com").await;
    let member_cookie = join(&app, &manager_cookie, "member@example.com").await;

    let req = test::TestRequest::delete()
        .uri(&format!("/households/{household_id}"))
        .insert_header(("Cookie", member_cookie))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 403);
}

// --- POST /households/{id}/join-code ---

#[sqlx::test]
async fn regenerate_join_code_replaces_the_old_one(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (cookie, id) = sign_in_with_household(&app, "sam@example.com").await;
    let me_req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let me: serde_json::Value = test::call_and_read_body_json(&app, me_req).await;
    let old_code = me["household"]["join_code"].clone();

    let req = test::TestRequest::post()
        .uri(&format!("/households/{id}/join-code"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let household: serde_json::Value = test::read_body_json(resp).await;
    assert!(household["join_code"].is_string());
    assert_ne!(household["join_code"], old_code);

    // The old code no longer lets anyone in.
    let cookie = sign_in(&app, "late@example.com").await;
    let join_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "first_name": "Late", "join_code": old_code }))
        .to_request();
    assert_eq!(test::call_service(&app, join_req).await.status(), 404);
}

#[sqlx::test]
async fn regenerate_join_code_requires_a_family_manager(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let (manager_cookie, id) = sign_in_with_household(&app, "sam@example.com").await;
    let member_cookie = join(&app, &manager_cookie, "member@example.com").await;

    let req = test::TestRequest::post()
        .uri(&format!("/households/{id}/join-code"))
        .insert_header(("Cookie", member_cookie))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 403);
}
