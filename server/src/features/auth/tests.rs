//! Integration tests for the auth HTTP API.
//!
//! Each `#[sqlx::test]` gets its own throwaway Postgres database (migrated from `migrations/`,
//! dropped afterwards), so these never touch real dev data.
//!
//! Every test runs with `skip_email_verification` on: that's the path that doesn't need a mail
//! provider, and it exercises the same token and session code a clicked link goes through.

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App};
use sqlx::PgPool;

use super::handlers::configure;
use super::tokens;
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

/// An `AuthConfig` that behaves like a real deployment: emails are sent, so `GET /auth/verify` is
/// the only way in. Nothing here reaches the network — `verify` never sends mail.
fn verifying_config() -> AuthConfig {
    AuthConfig {
        skip_email_verification: false,
        resend_api_key: Some("re_test".to_string()),
        ..test_config()
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
    app_with_config(pool, test_config())
}

fn app_with_config(
    pool: PgPool,
    config: AuthConfig,
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
        .app_data(web::Data::new(config))
        .app_data(web::Data::new(reqwest::Client::new()))
        .configure(configure)
}

/// Inserts a user and a magic-link token for them, returning the raw token that would have been
/// emailed. Lets the verify tests act as though a link was clicked.
async fn issue_login_token(pool: &PgPool, email: &str, expires_in_minutes: i64) -> String {
    let user_id: (i32,) = sqlx::query_as("INSERT INTO users (email) VALUES ($1) RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("insert user");

    let raw_token = tokens::generate();
    sqlx::query(
        "INSERT INTO login_tokens (user_id, token_hash, expires_at)
         VALUES ($1, $2, now() + make_interval(mins => $3))",
    )
    .bind(user_id.0)
    .bind(tokens::hash(&raw_token))
    .bind(expires_in_minutes as i32)
    .execute(pool)
    .await
    .expect("insert login token");

    raw_token
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

// --- POST /auth/request-login ---

#[sqlx::test]
async fn request_login_signs_in_directly_when_email_is_skipped(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::post()
        .uri("/auth/request-login")
        .set_json(serde_json::json!({ "email": "Sam@Example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    assert!(resp
        .response()
        .cookies()
        .any(|cookie| cookie.name() == SESSION_COOKIE_NAME));

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "signed_in");
    // Normalized on the way in, so "Sam@Example.com" and "sam@example.com" are one account.
    assert_eq!(body["session"]["user"]["email"], "sam@example.com");
    assert_eq!(body["session"]["households"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn request_login_reuses_the_user_on_a_second_sign_in(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let first = sign_in(&app, "sam@example.com").await;
    let second = sign_in(&app, "sam@example.com").await;
    assert_ne!(first, second, "each sign-in opens its own session");

    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", second))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(body["user"]["email"], "sam@example.com");
}

#[sqlx::test]
async fn request_login_sets_a_hardened_session_cookie(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::post()
        .uri("/auth/request-login")
        .set_json(serde_json::json!({ "email": "sam@example.com" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    let cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("a session cookie");

    // `build_session_cookie` is what these guarantee; asserting on the wire format is what
    // catches a regression there, rather than only in a unit test of the builder itself.
    assert_eq!(cookie.http_only(), Some(true));
    assert_eq!(cookie.same_site(), Some(actix_web::cookie::SameSite::Lax));
    assert_eq!(cookie.path(), Some("/"));
    // Not asserting `secure()`: this config is development, where it's deliberately off (a
    // `Secure` cookie is dropped over localhost's plain HTTP). Production is covered by
    // `AuthConfig::is_production` and its own tests in `shared::config`.
}

#[sqlx::test]
async fn request_login_rejects_an_unusable_email(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::post()
        .uri("/auth/request-login")
        .set_json(serde_json::json!({ "email": "not-an-email" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

// --- GET /auth/verify ---

#[sqlx::test]
async fn verify_rejects_an_unknown_token(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get()
        .uri("/auth/verify?token=00000000000000000000000000000000")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn verify_opens_a_session_and_marks_the_user_verified(pool: PgPool) {
    let raw_token = issue_login_token(&pool, "sam@example.com", 15).await;
    let app = test::init_service(app_with_config(pool, verifying_config())).await;

    let req = test::TestRequest::get()
        .uri(&format!("/auth/verify?token={raw_token}"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .expect("a session cookie");
    let cookie_header = format!("{}={}", cookie.name(), cookie.value());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["user"]["email"], "sam@example.com");
    // Clicking a real link proves the mailbox is theirs, unlike the dev shortcut.
    assert_eq!(body["user"]["status"], "verified");

    let me_req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie_header))
        .to_request();
    let me_resp = test::call_service(&app, me_req).await;
    assert_eq!(me_resp.status(), 200, "the new session works");
}

#[sqlx::test]
async fn verify_refuses_a_replayed_token(pool: PgPool) {
    let raw_token = issue_login_token(&pool, "sam@example.com", 15).await;
    let app = test::init_service(app_with_config(pool, verifying_config())).await;

    let first = test::TestRequest::get()
        .uri(&format!("/auth/verify?token={raw_token}"))
        .to_request();
    assert_eq!(test::call_service(&app, first).await.status(), 200);

    // Single use: a forwarded or re-clicked link is worthless once spent.
    let second = test::TestRequest::get()
        .uri(&format!("/auth/verify?token={raw_token}"))
        .to_request();
    assert_eq!(test::call_service(&app, second).await.status(), 400);
}

#[sqlx::test]
async fn verify_refuses_an_expired_token(pool: PgPool) {
    let raw_token = issue_login_token(&pool, "sam@example.com", -1).await;
    let app = test::init_service(app_with_config(pool, verifying_config())).await;

    let req = test::TestRequest::get()
        .uri(&format!("/auth/verify?token={raw_token}"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn verify_leaves_status_unchanged_when_email_verification_is_skipped(pool: PgPool) {
    // Reachable if someone hits `GET /auth/verify` directly while the dev shortcut is on — e.g. a
    // stale link from before the flag was flipped. It must not claim the mailbox was proven.
    let raw_token = issue_login_token(&pool, "sam@example.com", 15).await;
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get()
        .uri(&format!("/auth/verify?token={raw_token}"))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["user"]["status"], "pending");
}

#[sqlx::test]
async fn dev_shortcut_spends_the_token_it_issues(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    sign_in(&app, "sam@example.com").await;

    let used: (bool,) = sqlx::query_as("SELECT used_at IS NOT NULL FROM login_tokens LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("a login token was stored");

    assert!(used.0, "the auto-verified token can't be replayed later");
}

// --- GET /auth/me ---

#[sqlx::test]
async fn me_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get().uri("/auth/me").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn me_rejects_an_unknown_session_cookie(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", format!("{SESSION_COOKIE_NAME}=made-up")))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn me_rejects_a_suspended_users_session(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    // A suspension takes effect immediately: the session row is still live and unexpired, only
    // the account behind it changed.
    sqlx::query("UPDATE users SET status = 'suspended' WHERE email = 'sam@example.com'")
        .execute(&pool)
        .await
        .expect("suspend the user");

    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

// --- POST /auth/logout ---

#[sqlx::test]
async fn logout_ends_the_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let logout_req = test::TestRequest::post()
        .uri("/auth/logout")
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let logout_resp = test::call_service(&app, logout_req).await;
    assert_eq!(logout_resp.status(), 204);

    let me_req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    let me_resp = test::call_service(&app, me_req).await;
    assert_eq!(me_resp.status(), 401, "the session no longer resolves");
}

#[sqlx::test]
async fn logout_without_a_session_still_succeeds(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::post().uri("/auth/logout").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 204);
}

// --- POST /auth/onboarding ---

#[sqlx::test]
async fn onboarding_creates_a_household_and_makes_the_caller_its_manager(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(resp).await;
    let households = body["households"].as_array().unwrap();
    assert_eq!(households.len(), 1);
    assert_eq!(households[0]["type"], "family_manager");
    assert_eq!(
        households[0]["join_code"].as_str().unwrap().len(),
        8,
        "a join code is generated for the new household"
    );
}

#[sqlx::test]
async fn onboarding_joins_an_existing_household_by_join_code(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let manager_cookie = sign_in(&app, "manager@example.com").await;
    let create_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", manager_cookie))
        .set_json(serde_json::json!({}))
        .to_request();
    let created: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let join_code = created["households"][0]["join_code"].as_str().unwrap();

    let member_cookie = sign_in(&app, "member@example.com").await;
    let join_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", member_cookie))
        .set_json(serde_json::json!({ "join_code": join_code }))
        .to_request();
    let join_resp = test::call_service(&app, join_req).await;

    assert_eq!(join_resp.status(), 201);
    let body: serde_json::Value = test::read_body_json(join_resp).await;
    let households = body["households"].as_array().unwrap();
    assert_eq!(households.len(), 1);
    assert_eq!(households[0]["type"], "family_member");
    assert_eq!(
        households[0]["household_id"], created["households"][0]["household_id"],
        "joined the manager's household, not a new one"
    );
}

#[sqlx::test]
async fn onboarding_accepts_a_lowercase_join_code(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let manager_cookie = sign_in(&app, "manager@example.com").await;
    let create_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", manager_cookie))
        .set_json(serde_json::json!({}))
        .to_request();
    let created: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let join_code = created["households"][0]["join_code"]
        .as_str()
        .unwrap()
        .to_lowercase();

    let member_cookie = sign_in(&app, "member@example.com").await;
    let join_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", member_cookie))
        .set_json(serde_json::json!({ "join_code": format!("  {join_code}  ") }))
        .to_request();
    let join_resp = test::call_service(&app, join_req).await;

    assert_eq!(join_resp.status(), 201);
}

#[sqlx::test]
async fn onboarding_rejects_an_unknown_join_code(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "join_code": "ZZZZZZZZ" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn onboarding_rejects_a_blank_join_code_instead_of_creating_a_household(pool: PgPool) {
    // A `join_code` field that's present but empty (or whitespace) is a mistake, not a request
    // to create a new household — the two must not be conflated.
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "join_code": "   " }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn onboarding_rejects_joining_the_same_household_twice(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in(&app, "sam@example.com").await;

    let create_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({}))
        .to_request();
    let created: serde_json::Value = test::call_and_read_body_json(&app, create_req).await;
    let join_code = created["households"][0]["join_code"].as_str().unwrap();

    let rejoin_req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "join_code": join_code }))
        .to_request();
    let rejoin_resp = test::call_service(&app, rejoin_req).await;

    assert_eq!(rejoin_resp.status(), 409);
}

#[sqlx::test]
async fn onboarding_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}
