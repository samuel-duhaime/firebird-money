//! Integration tests for the accounts HTTP API. Each `#[sqlx::test]` gets its own throwaway
//! database.

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

/// Creates a transaction in `account_id`, in the household's first seeded category.
async fn create_transaction<S, B>(app: &S, cookie: &str, pool: &PgPool, account_id: i64)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let (category_id,): (i32,) = sqlx::query_as(
        "SELECT c.id FROM categories c
         JOIN accounts a ON a.household_id = c.household_id
         WHERE a.id = $1 LIMIT 1",
    )
    .bind(account_id as i32)
    .fetch_one(pool)
    .await
    .unwrap();

    let (status, body) = call(
        app,
        cookie,
        test::TestRequest::post().uri("/transactions"),
        Some(serde_json::json!({
            "date": "2026-10-01",
            "original_statement": "STARBUCKS",
            "amount": "5.00",
            "category_id": category_id,
            "account_id": account_id,
        })),
    )
    .await;
    assert_eq!(status, 201, "{body}");
}

#[sqlx::test]
async fn create_account_returns_the_row_with_its_institution(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let institution_id = create_institution(&app, &cookie, "Caisse du coin").await;

    let (status, body) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/accounts"),
        Some(serde_json::json!({
            "institution_id": institution_id,
            "name": "  Compte conjoint ",
            "type": "checking",
            "last_four": "5012",
            "currency": "cad",
        })),
    )
    .await;

    assert_eq!(status, 201, "{body}");
    assert_eq!(body["name"], "Compte conjoint");
    assert_eq!(body["institution_id"], institution_id);
    assert_eq!(body["institution_name"], "Caisse du coin");
    assert_eq!(body["type"], "checking");
    assert_eq!(body["last_four"], "5012");
    assert_eq!(body["currency"], "CAD");
    assert_eq!(body["transaction_count"], 0);
}

#[sqlx::test]
async fn create_account_defaults_currency_and_blank_last_four(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let institution_id = create_institution(&app, &cookie, "Caisse du coin").await;

    let (status, body) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/accounts"),
        Some(serde_json::json!({
            "institution_id": institution_id,
            "name": "Cash",
            "type": "cash",
            "last_four": "  ",
        })),
    )
    .await;

    assert_eq!(status, 201, "{body}");
    assert_eq!(body["currency"], "CAD");
    assert_eq!(body["last_four"], serde_json::Value::Null);
}

#[sqlx::test]
async fn create_account_can_use_a_common_institution(pool: PgPool) {
    crate::features::institutions::repository::seed_defaults(&pool)
        .await
        .unwrap();
    let (institution_id,): (i32,) =
        sqlx::query_as("SELECT id FROM institutions WHERE name = 'Desjardins'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let account_id = create_account(&app, &cookie, institution_id as i64, "Chèques").await;

    let (_, body) = call(
        &app,
        &cookie,
        test::TestRequest::get().uri(&format!("/accounts/{account_id}")),
        None,
    )
    .await;
    assert_eq!(body["institution_name"], "Desjardins");
}

#[sqlx::test]
async fn create_account_rejects_invalid_fields(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let institution_id = create_institution(&app, &cookie, "Caisse du coin").await;

    for (field, value) in [
        ("name", serde_json::json!(" ")),
        ("type", serde_json::json!("piggy_bank")),
        ("last_four", serde_json::json!("12345")),
        ("currency", serde_json::json!("dollars")),
    ] {
        let mut body = serde_json::json!({
            "institution_id": institution_id,
            "name": "Chèques",
            "type": "checking",
        });
        body[field] = value;
        let (status, _) = call(
            &app,
            &cookie,
            test::TestRequest::post().uri("/accounts"),
            Some(body),
        )
        .await;
        assert_eq!(status, 400, "{field}");
    }
}

#[sqlx::test]
async fn create_account_rejects_another_households_institution(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let institution_id = create_institution(&app, &cookie_a, "A's Caisse").await;

    let (status, _) = call(
        &app,
        &cookie_b,
        test::TestRequest::post().uri("/accounts"),
        Some(serde_json::json!({
            "institution_id": institution_id,
            "name": "Sneaky",
            "type": "checking",
        })),
    )
    .await;

    assert_eq!(status, 400);
}

#[sqlx::test]
async fn accounts_do_not_leak_between_households(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie_a = sign_in_with_household(&app, "a@example.com").await;
    let cookie_b = sign_in_with_household(&app, "b@example.com").await;
    let institution_id = create_institution(&app, &cookie_a, "A's Caisse").await;
    let account_id = create_account(&app, &cookie_a, institution_id, "A's account").await;

    let (_, list) = call(
        &app,
        &cookie_b,
        test::TestRequest::get().uri("/accounts"),
        None,
    )
    .await;
    assert_eq!(list.as_array().unwrap().len(), 0);

    for req in [
        test::TestRequest::get().uri(&format!("/accounts/{account_id}")),
        test::TestRequest::delete().uri(&format!("/accounts/{account_id}")),
    ] {
        let (status, _) = call(&app, &cookie_b, req, None).await;
        assert_eq!(status, 404);
    }
}

#[sqlx::test]
async fn list_accounts_requires_a_session(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;

    let req = test::TestRequest::get().uri("/accounts").to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 401);
}

#[sqlx::test]
async fn list_accounts_groups_by_institution(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let zeta = create_institution(&app, &cookie, "Zeta").await;
    let alpha = create_institution(&app, &cookie, "Alpha").await;
    create_account(&app, &cookie, zeta, "B").await;
    create_account(&app, &cookie, alpha, "Z").await;
    create_account(&app, &cookie, zeta, "A").await;

    let (_, list) = call(
        &app,
        &cookie,
        test::TestRequest::get().uri("/accounts"),
        None,
    )
    .await;

    let names: Vec<(String, String)> = list
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["institution_name"].as_str().unwrap().to_string(),
                a["name"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    assert_eq!(
        names,
        vec![
            ("Alpha".to_string(), "Z".to_string()),
            ("Zeta".to_string(), "A".to_string()),
            ("Zeta".to_string(), "B".to_string()),
        ]
    );
}

#[sqlx::test]
async fn update_account_changes_fields_and_clears_last_four(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let first = create_institution(&app, &cookie, "First").await;
    let second = create_institution(&app, &cookie, "Second").await;
    let (_, created) = call(
        &app,
        &cookie,
        test::TestRequest::post().uri("/accounts"),
        Some(serde_json::json!({
            "institution_id": first,
            "name": "Visa",
            "type": "credit_card",
            "last_four": "3010",
        })),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, body) = call(
        &app,
        &cookie,
        test::TestRequest::patch().uri(&format!("/accounts/{id}")),
        Some(serde_json::json!({
            "institution_id": second,
            "name": "Mastercard",
            "last_four": "",
            "currency": "usd",
        })),
    )
    .await;

    assert_eq!(status, 200, "{body}");
    assert_eq!(body["institution_name"], "Second");
    assert_eq!(body["name"], "Mastercard");
    assert_eq!(body["type"], "credit_card");
    assert_eq!(body["last_four"], serde_json::Value::Null);
    assert_eq!(body["currency"], "USD");
}

#[sqlx::test]
async fn update_account_404s_for_an_unknown_id(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::patch().uri("/accounts/999999"),
        Some(serde_json::json!({ "name": "Nope" })),
    )
    .await;

    assert_eq!(status, 404);
}

#[sqlx::test]
async fn delete_account_is_blocked_while_transactions_use_it(pool: PgPool) {
    let app = test::init_service(app_with(pool.clone())).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let institution_id = create_institution(&app, &cookie, "Caisse du coin").await;
    let account_id = create_account(&app, &cookie, institution_id, "Chèques").await;
    create_transaction(&app, &cookie, &pool, account_id).await;

    let (_, account) = call(
        &app,
        &cookie,
        test::TestRequest::get().uri(&format!("/accounts/{account_id}")),
        None,
    )
    .await;
    assert_eq!(account["transaction_count"], 1);

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::delete().uri(&format!("/accounts/{account_id}")),
        None,
    )
    .await;
    assert_eq!(status, 409);
}

#[sqlx::test]
async fn delete_account_removes_an_unused_account(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "sam@example.com").await;
    let institution_id = create_institution(&app, &cookie, "Caisse du coin").await;
    let account_id = create_account(&app, &cookie, institution_id, "Chèques").await;

    let (status, _) = call(
        &app,
        &cookie,
        test::TestRequest::delete().uri(&format!("/accounts/{account_id}")),
        None,
    )
    .await;
    assert_eq!(status, 204);
}
