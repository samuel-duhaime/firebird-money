//! Integration tests for the members HTTP API. Every route is scoped to the caller's
//! own household — see `handlers.rs`.
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
        .configure(crate::features::households::configure)
        .configure(crate::features::users::configure)
        .configure(crate::features::categories::configure)
        .configure(crate::features::institutions::configure)
        .configure(crate::features::accounts::configure)
        .configure(crate::features::transactions::configure)
}

/// Signs in as `email` and completes onboarding, returning the session cookie: with a
/// `join_code` they join that household as a `family_member`; without one they create a new
/// household and become its `family_manager`.
async fn onboard<S, B>(app: &S, email: &str, join_code: Option<&str>) -> String
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
        .set_json(serde_json::json!({
            "first_name": "Test",
            "last_name": "User",
            "join_code": join_code,
        }))
        .to_request();
    assert_eq!(test::call_service(app, onboard_req).await.status(), 201);

    cookie
}

/// `GET /auth/me` for `cookie`.
async fn me<S, B>(app: &S, cookie: &str) -> serde_json::Value
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/auth/me")
        .insert_header(("Cookie", cookie))
        .to_request();
    test::call_and_read_body_json(app, req).await
}

/// Has `email` join `manager_cookie`'s household by its code, returning the new member's cookie and
/// user id.
async fn join<S, B>(app: &S, manager_cookie: &str, email: &str) -> (String, i64)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let join_code = me(app, manager_cookie).await["household"]["join_code"]
        .as_str()
        .unwrap()
        .to_string();
    let cookie = onboard(app, email, Some(&join_code)).await;
    let id = me(app, &cookie).await["user"]["id"].as_i64().unwrap();
    (cookie, id)
}

// --- GET /members ---

#[sqlx::test]
async fn list_members_returns_only_the_callers_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;

    let req = test::TestRequest::get()
        .uri("/members")
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    // Only B themselves — not the manager or Jane from A's household.
    assert_eq!(rows.len(), 1);
    assert!(!rows.iter().any(|r| r["id"].as_i64() == Some(jane_id)));
}

#[sqlx::test]
async fn list_members_includes_everyone_who_joined(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::get()
        .uri("/members")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["id"].as_i64().unwrap(), jane_id);
    assert_eq!(rows[1]["role"], "family_member");
}

#[sqlx::test]
async fn list_members_includes_each_members_user_details(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;

    let req = test::TestRequest::get()
        .uri("/members")
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["email"], "sam@example.com");
    assert_eq!(rows[0]["first_name"], "Test");
    assert_eq!(rows[0]["last_name"], "User");
    assert_eq!(rows[0]["status"], "pending");
    assert_eq!(rows[0]["role"], "family_manager");
}

#[sqlx::test]
async fn list_members_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get().uri("/members").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

// --- GET /members/{id} ---

#[sqlx::test]
async fn get_member_returns_the_member(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::get()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["id"].as_i64().unwrap(), jane_id);
    assert_eq!(body["email"], "jane@example.com");
    assert_eq!(body["role"], "family_member");
}

#[sqlx::test]
async fn get_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;

    let req = test::TestRequest::get()
        .uri("/members/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn get_member_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;
    let req = test::TestRequest::get()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /members/{id} ---

#[sqlx::test]
async fn update_member_changes_role(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (jane_cookie, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "role": "family_manager" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["role"], "family_manager");
    assert_eq!(
        me(&app, &jane_cookie).await["household"]["role"],
        "family_manager"
    );
}

#[sqlx::test]
async fn update_member_rejects_invalid_role(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "role": "nonsense" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn update_member_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;
    let req = test::TestRequest::patch()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie_b))
        .set_json(serde_json::json!({ "role": "family_manager" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- DELETE /members/{id} ---

#[sqlx::test]
async fn delete_member_removes_them_from_the_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (jane_cookie, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    assert_eq!(test::call_service(&app, delete_req).await.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    assert_eq!(test::call_service(&app, get_req).await.status(), 404);

    // Her account is kept, just without a household.
    let jane = me(&app, &jane_cookie).await;
    assert_eq!(jane["user"]["id"].as_i64().unwrap(), jane_id);
    assert!(jane["household"].is_null());
}

#[sqlx::test]
async fn delete_member_is_blocked_while_they_have_transactions(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;
    let (jane_cookie, jane_id) = join(&app, &cookie, "jane@example.com").await;

    let categories_req = test::TestRequest::get()
        .uri("/categories")
        .insert_header(("Cookie", jane_cookie.clone()))
        .to_request();
    let categories: serde_json::Value = test::call_and_read_body_json(&app, categories_req).await;
    let category_id = categories[0]["id"].as_i64().unwrap();
    let account_id = crate::shared::test_support::test_account_id(&app, &jane_cookie).await;

    let transaction_req = test::TestRequest::post()
        .uri("/transactions")
        .insert_header(("Cookie", jane_cookie))
        .set_json(serde_json::json!({
            "date": "2026-10-15",
            "original_statement": "STARBUCKS",
            "amount": "4.50",
            "category_id": category_id,
            "account_id": account_id,
        }))
        .to_request();
    assert_eq!(
        test::call_service(&app, transaction_req).await.status(),
        201
    );

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;

    assert_eq!(resp.status(), 409);
}

#[sqlx::test]
async fn delete_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "sam@example.com", None).await;

    let req = test::TestRequest::delete()
        .uri("/members/999999")
        .insert_header(("Cookie", cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

#[sqlx::test]
async fn delete_member_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::delete()
        .uri("/members/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 401);
}

#[sqlx::test]
async fn delete_member_from_another_household_is_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = onboard(&app, "a@example.com", None).await;
    let (_, jane_id) = join(&app, &cookie_a, "jane@example.com").await;

    let cookie_b = onboard(&app, "b@example.com", None).await;
    let delete_req = test::TestRequest::delete()
        .uri(&format!("/members/{jane_id}"))
        .insert_header(("Cookie", cookie_b))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;

    assert_eq!(resp.status(), 404);
}

// --- Managers only ---

#[sqlx::test]
async fn update_member_requires_a_family_manager(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let manager_cookie = onboard(&app, "sam@example.com", None).await;
    let manager_id = me(&app, &manager_cookie).await["user"]["id"]
        .as_i64()
        .unwrap();
    let (jane_cookie, _) = join(&app, &manager_cookie, "jane@example.com").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/members/{manager_id}"))
        .insert_header(("Cookie", jane_cookie))
        .set_json(serde_json::json!({ "role": "family_member" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 403);
    assert_eq!(
        me(&app, &manager_cookie).await["household"]["role"],
        "family_manager"
    );
}

#[sqlx::test]
async fn delete_member_requires_a_family_manager(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let manager_cookie = onboard(&app, "sam@example.com", None).await;
    let manager_id = me(&app, &manager_cookie).await["user"]["id"]
        .as_i64()
        .unwrap();
    let (jane_cookie, _) = join(&app, &manager_cookie, "jane@example.com").await;

    let req = test::TestRequest::delete()
        .uri(&format!("/members/{manager_id}"))
        .insert_header(("Cookie", jane_cookie))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 403);
    assert!(!me(&app, &manager_cookie).await["household"].is_null());
}

// --- /members/{id}/picture ---

/// The smallest bytes each accepted type is recognized by, enough for the server's type check.
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest of a png";
const JPEG: &[u8] = b"\xFF\xD8\xFF\xE0rest of a jpeg";
const WEBP: &[u8] = b"RIFF\x00\x00\x00\x00WEBPrest of a webp";

/// `PUT /members/{id}/picture` for `cookie` with `file` as the multipart body, claiming to be a PNG
/// whatever it really is.
async fn put_picture<S, B>(app: &S, cookie: &str, id: i64, file: &[u8]) -> ServiceResponse<B>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let boundary = "picture-test-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"me.png\"\r\n\
         Content-Type: image/png\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(file);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let req = test::TestRequest::put()
        .uri(&format!("/members/{id}/picture"))
        .insert_header(("Cookie", cookie))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        ))
        .set_payload(body)
        .to_request();
    test::call_service(app, req).await
}

/// `GET /members/{id}/picture` for `cookie`.
async fn get_picture<S, B>(app: &S, cookie: &str, id: i64) -> ServiceResponse<B>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri(&format!("/members/{id}/picture"))
        .insert_header(("Cookie", cookie))
        .to_request();
    test::call_service(app, req).await
}

/// `DELETE /members/{id}/picture` for `cookie`.
async fn delete_picture<S, B>(app: &S, cookie: &str, id: i64) -> ServiceResponse<B>
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::delete()
        .uri(&format!("/members/{id}/picture"))
        .insert_header(("Cookie", cookie))
        .to_request();
    test::call_service(app, req).await
}

/// The signed-in user's id.
async fn my_id<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    me(app, cookie).await["user"]["id"].as_i64().unwrap()
}

#[sqlx::test]
async fn a_member_has_no_picture_until_they_add_one(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    assert!(me(&app, &cookie).await["user"]["picture_url"].is_null());
    assert_eq!(get_picture(&app, &cookie, id).await.status(), 404);
}

#[sqlx::test]
async fn an_uploaded_picture_is_served_back_with_its_real_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    // `put_picture` claims every upload is a PNG: the stored type must come from the bytes.
    for (bytes, content_type) in [
        (PNG, "image/png"),
        (JPEG, "image/jpeg"),
        (WEBP, "image/webp"),
    ] {
        let resp = put_picture(&app, &cookie, id, bytes).await;
        assert_eq!(resp.status(), 200);

        let resp = get_picture(&app, &cookie, id).await;
        assert_eq!(resp.status(), 200);
        let header = |name: &str| {
            resp.headers()
                .get(name)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(header("Content-Type"), content_type);
        assert_eq!(header("X-Content-Type-Options"), "nosniff");
        assert!(header("Cache-Control").contains("private"));
        assert_eq!(&test::read_body(resp).await[..], bytes);
    }
}

#[sqlx::test]
async fn the_picture_url_appears_in_members_and_auth_me_and_changes_with_each_upload(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    let resp = put_picture(&app, &cookie, id, PNG).await;
    let member: serde_json::Value = test::read_body_json(resp).await;
    let first_url = member["picture_url"].as_str().unwrap().to_string();
    assert!(first_url.starts_with(&format!("/members/{id}/picture?v=")));
    assert_eq!(me(&app, &cookie).await["user"]["picture_url"], first_url);

    let req = test::TestRequest::get()
        .uri("/members")
        .insert_header(("Cookie", cookie.clone()))
        .to_request();
    let members: serde_json::Value = test::call_and_read_body_json(&app, req).await;
    assert_eq!(members[0]["picture_url"], first_url);

    // Millisecond versions: make sure the second upload lands on a later one.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let resp = put_picture(&app, &cookie, id, JPEG).await;
    let member: serde_json::Value = test::read_body_json(resp).await;
    assert_ne!(member["picture_url"], first_url);
}

#[sqlx::test]
async fn refuses_a_file_that_is_not_an_accepted_image(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    for bytes in [
        b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>".as_slice(),
        b"GIF89a".as_slice(),
        b"RIFF\x00\x00\x00\x00WAVE".as_slice(),
        b"".as_slice(),
    ] {
        let resp = put_picture(&app, &cookie, id, bytes).await;
        assert_eq!(resp.status(), 400);
        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(
            body["error"],
            "The picture must be a PNG, JPEG, or WebP image"
        );
    }
    assert_eq!(get_picture(&app, &cookie, id).await.status(), 404);
}

#[sqlx::test]
async fn refuses_a_picture_over_the_size_limit(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    let mut too_large = PNG.to_vec();
    too_large.resize(3 * 1024 * 1024, 0);

    let resp = put_picture(&app, &cookie, id, &too_large).await;
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "The picture is too large (2 MB max)");
    assert_eq!(get_picture(&app, &cookie, id).await.status(), 404);
}

#[sqlx::test]
async fn refuses_a_picture_just_over_the_size_limit(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    let mut just_over = PNG.to_vec();
    just_over.resize(2_000_001, 0);

    assert_eq!(
        put_picture(&app, &cookie, id, &just_over).await.status(),
        400
    );
}

#[sqlx::test]
async fn a_picture_upload_waits_for_a_removal_in_progress_and_then_fails(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let manager = onboard(&app, "manager@example.com", None).await;
    let (_, member_id) = join(&app, &manager, "member@example.com").await;
    let household_id = me(&app, &manager).await["household"]["household_id"]
        .as_i64()
        .unwrap() as i32;

    // A removal that has changed the row but not committed yet, so the upload's first look at the
    // member still finds them in the household.
    let mut removal = pool.begin().await.unwrap();
    sqlx::query("UPDATE users SET household_id = NULL, household_role = NULL WHERE id = $1")
        .bind(member_id as i32)
        .execute(&mut *removal)
        .await
        .unwrap();

    let upload = tokio::spawn({
        let pool = pool.clone();
        async move {
            super::repository::set_picture(&pool, household_id, member_id as i32, "image/png", PNG)
                .await
        }
    });
    actix_web::rt::time::sleep(std::time::Duration::from_millis(100)).await;
    removal.commit().await.unwrap();

    assert!(!upload.await.unwrap().unwrap());
}

#[sqlx::test]
async fn refuses_an_upload_without_a_file(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    let boundary = "picture-test-boundary";
    let req = test::TestRequest::put()
        .uri(&format!("/members/{id}/picture"))
        .insert_header(("Cookie", cookie))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={boundary}"),
        ))
        .set_payload(format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"other\"\r\n\r\nhi\r\n--{boundary}--\r\n"
        ))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(
        body["error"],
        "The picture must be a PNG, JPEG, or WebP image"
    );
}

#[sqlx::test]
async fn deleting_a_picture_removes_it_and_can_be_repeated(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;
    put_picture(&app, &cookie, id, PNG).await;

    for _ in 0..2 {
        let resp = delete_picture(&app, &cookie, id).await;
        assert_eq!(resp.status(), 200);
        let member: serde_json::Value = test::read_body_json(resp).await;
        assert!(member["picture_url"].is_null());
    }
    assert_eq!(get_picture(&app, &cookie, id).await.status(), 404);
}

#[sqlx::test]
async fn a_member_can_change_only_their_own_picture(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let manager = onboard(&app, "manager@example.com", None).await;
    let manager_id = my_id(&app, &manager).await;
    let (member, member_id) = join(&app, &manager, "member@example.com").await;
    put_picture(&app, &manager, manager_id, PNG).await;

    assert_eq!(
        put_picture(&app, &member, member_id, PNG).await.status(),
        200
    );
    assert_eq!(
        put_picture(&app, &member, manager_id, JPEG).await.status(),
        403
    );
    assert_eq!(
        delete_picture(&app, &member, manager_id).await.status(),
        403
    );

    // The manager's picture came through both refused requests untouched.
    let resp = get_picture(&app, &member, manager_id).await;
    assert_eq!(&test::read_body(resp).await[..], PNG);
}

#[sqlx::test]
async fn a_manager_can_change_anyone_in_their_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let manager = onboard(&app, "manager@example.com", None).await;
    let (member, member_id) = join(&app, &manager, "member@example.com").await;

    assert_eq!(
        put_picture(&app, &manager, member_id, PNG).await.status(),
        200
    );
    assert_eq!(get_picture(&app, &member, member_id).await.status(), 200);
    assert_eq!(
        delete_picture(&app, &manager, member_id).await.status(),
        200
    );
    assert_eq!(get_picture(&app, &member, member_id).await.status(), 404);
}

#[sqlx::test]
async fn pictures_stay_inside_the_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let manager_a = onboard(&app, "a@example.com", None).await;
    let a_id = my_id(&app, &manager_a).await;
    let manager_b = onboard(&app, "b@example.com", None).await;
    put_picture(&app, &manager_a, a_id, PNG).await;

    assert_eq!(get_picture(&app, &manager_b, a_id).await.status(), 404);
    assert_eq!(
        put_picture(&app, &manager_b, a_id, JPEG).await.status(),
        404
    );
    assert_eq!(delete_picture(&app, &manager_b, a_id).await.status(), 404);

    let resp = get_picture(&app, &manager_a, a_id).await;
    assert_eq!(&test::read_body(resp).await[..], PNG);
}

#[sqlx::test]
async fn picture_routes_require_signing_in(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get()
        .uri("/members/1/picture")
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 401);
}

#[sqlx::test]
async fn a_removed_member_keeps_their_picture_but_the_household_no_longer_sees_it(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let manager = onboard(&app, "manager@example.com", None).await;
    let (member, member_id) = join(&app, &manager, "member@example.com").await;
    put_picture(&app, &member, member_id, PNG).await;

    let req = test::TestRequest::delete()
        .uri(&format!("/members/{member_id}"))
        .insert_header(("Cookie", manager.clone()))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 204);

    assert_eq!(get_picture(&app, &manager, member_id).await.status(), 404);
    assert!(me(&app, &member).await["user"]["picture_url"].is_string());
}

#[sqlx::test]
async fn patching_a_user_can_no_longer_set_a_picture(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = onboard(&app, "a@example.com", None).await;
    let id = my_id(&app, &cookie).await;

    let req = test::TestRequest::patch()
        .uri(&format!("/users/{id}"))
        .insert_header(("Cookie", cookie.clone()))
        .set_json(serde_json::json!({
            "avatar_url": "https://example.com/tracker.png",
            "picture_url": "https://example.com/tracker.png",
        }))
        .to_request();
    let user: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert!(user["picture_url"].is_null());
    assert!(user.get("avatar_url").is_none());
}
