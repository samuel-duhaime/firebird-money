//! Integration tests for the rules HTTP API.
//!
//! Each `#[sqlx::test]` gets its own throwaway Postgres database (migrated from `migrations/`,
//! dropped afterwards), so these never touch real dev data.

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App};
use serde_json::{json, Value};
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
        .configure(crate::features::transactions::configure)
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
        .set_json(json!({ "email": email }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), 200);
    let cookie = resp
        .response()
        .cookies()
        .find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
        .unwrap_or_else(|| panic!("expected a {SESSION_COOKIE_NAME} cookie"));
    let cookie = format!("{}={}", cookie.name(), cookie.value());

    let req = test::TestRequest::post()
        .uri("/auth/onboarding")
        .insert_header(("Cookie", cookie.clone()))
        .set_json(json!({ "first_name": "Test", "last_name": "User" }))
        .to_request();
    assert_eq!(test::call_service(app, req).await.status(), 201);
    cookie
}

/// Sends `method uri` with `body` as the signed-in caller, returning the status and JSON body
/// (`Null` when there's none).
async fn call<S, B>(app: &S, cookie: &str, method: &str, uri: &str, body: Value) -> (u16, Value)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = match method {
        "GET" => test::TestRequest::get(),
        "POST" => test::TestRequest::post(),
        "PATCH" => test::TestRequest::patch(),
        "DELETE" => test::TestRequest::delete(),
        _ => unreachable!(),
    }
    .uri(uri)
    .insert_header(("Cookie", cookie));
    let req = match body {
        Value::Null => req.to_request(),
        body => req.set_json(body).to_request(),
    };
    let resp = test::call_service(app, req).await;
    let status = resp.status().as_u16();
    let bytes = test::read_body(resp).await;
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

/// The ids of the household's first two seeded categories and first seeded tag.
async fn seeded_ids<S, B>(app: &S, cookie: &str) -> (i64, i64, i64)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let (_, categories) = call(app, cookie, "GET", "/categories", Value::Null).await;
    let (_, tags) = call(app, cookie, "GET", "/tags", Value::Null).await;
    (
        categories[0]["id"].as_i64().unwrap(),
        categories[1]["id"].as_i64().unwrap(),
        tags[0]["id"].as_i64().unwrap(),
    )
}

/// Creates a transaction with `statement` as its raw text, returning it.
async fn create_transaction<S, B>(
    app: &S,
    cookie: &str,
    category_id: i64,
    statement: &str,
    amount: &str,
) -> Value
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let account_id = crate::shared::test_support::test_account_id(app, cookie).await;
    let (status, body) = call(
        app,
        cookie,
        "POST",
        "/transactions",
        json!({
            "date": "2026-11-01",
            "original_statement": statement,
            "amount": amount,
            "category_id": category_id,
            "account_id": account_id,
        }),
    )
    .await;
    assert_eq!(status, 201, "{body}");
    body
}

#[sqlx::test]
async fn crud_and_reorder(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "rules-crud@example.com").await;
    let (category_a, category_b, _) = seeded_ids(&app, &cookie).await;

    let new_rule = |text: &str, category_id: i64| {
        json!({
            "conditions": { "original_statement": [{ "operator": "contains", "value": text }] },
            "actions": { "category_id": category_id },
        })
    };
    let (status, first) = call(
        &app,
        &cookie,
        "POST",
        "/rules",
        new_rule("spotify", category_a),
    )
    .await;
    assert_eq!(status, 201, "{first}");
    assert_eq!(first["sort_order"], 0);
    assert_eq!(first["conditions"]["merchant_ids"], Value::Null);
    let (_, second) = call(
        &app,
        &cookie,
        "POST",
        "/rules",
        new_rule("netflix", category_b),
    )
    .await;
    assert_eq!(second["sort_order"], 1);

    let (status, rules) = call(
        &app,
        &cookie,
        "PATCH",
        "/rules/reorder",
        json!({ "rule_ids": [second["id"], first["id"]] }),
    )
    .await;
    assert_eq!(status, 200);
    assert_eq!(rules[0]["id"], second["id"]);

    let uri = format!("/rules/{}", first["id"]);
    let (status, updated) = call(
        &app,
        &cookie,
        "PATCH",
        &uri,
        json!({ "actions": { "category_id": category_b, "owner": "shared" } }),
    )
    .await;
    assert_eq!(status, 200, "{updated}");
    assert_eq!(updated["actions"]["owner"], "shared");
    assert_eq!(updated["conditions"], first["conditions"]);

    let (status, _) = call(&app, &cookie, "DELETE", &uri, Value::Null).await;
    assert_eq!(status, 204);
    let (status, _) = call(&app, &cookie, "GET", &uri, Value::Null).await;
    assert_eq!(status, 404);
}

#[sqlx::test]
async fn rejects_rules_that_cant_be_saved(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "rules-invalid@example.com").await;
    let (category_id, _, _) = seeded_ids(&app, &cookie).await;
    let other = sign_in_with_household(&app, "rules-invalid-other@example.com").await;
    let (other_category_id, _, _) = seeded_ids(&app, &other).await;

    for body in [
        json!({ "conditions": {}, "actions": { "category_id": category_id } }),
        json!({ "conditions": { "merchant_ids": [] }, "actions": { "category_id": category_id } }),
        json!({
            "conditions": { "original_statement": [{ "operator": "contains", "value": " " }] },
            "actions": { "category_id": category_id },
        }),
        json!({
            "conditions": { "amount": { "operator": "between", "value": "20", "value_max": "10" } },
            "actions": { "category_id": category_id },
        }),
        json!({ "conditions": { "category_ids": [category_id] }, "actions": {} }),
        json!({
            "conditions": { "category_ids": [category_id] },
            "actions": { "category_id": other_category_id },
        }),
    ] {
        let (status, response) = call(&app, &cookie, "POST", "/rules", body.clone()).await;
        assert_eq!(status, 400, "{body} -> {response}");
    }
}

#[sqlx::test]
async fn rules_stay_in_their_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "rules-mine@example.com").await;
    let (category_id, _, _) = seeded_ids(&app, &cookie).await;
    let (_, rule) = call(
        &app,
        &cookie,
        "POST",
        "/rules",
        json!({ "conditions": { "category_ids": [category_id] }, "actions": { "owner": "shared" } }),
    )
    .await;

    let other = sign_in_with_household(&app, "rules-theirs@example.com").await;
    let uri = format!("/rules/{}", rule["id"]);
    assert_eq!(call(&app, &other, "GET", &uri, Value::Null).await.0, 404);
    assert_eq!(call(&app, &other, "DELETE", &uri, Value::Null).await.0, 404);
    let (_, rules) = call(&app, &other, "GET", "/rules", Value::Null).await;
    assert_eq!(rules, json!([]));
}

#[sqlx::test]
async fn preview_then_apply_to_existing(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let cookie = sign_in_with_household(&app, "rules-apply@example.com").await;
    let (category_a, category_b, tag_id) = seeded_ids(&app, &cookie).await;
    let spotify = create_transaction(&app, &cookie, category_a, "SPOTIFY P1234", "12.99").await;
    create_transaction(&app, &cookie, category_b, "Spotify family", "19.99").await;
    let netflix = create_transaction(&app, &cookie, category_a, "NETFLIX.COM", "16.99").await;

    let rule = json!({
        "conditions": {
            "original_statement": [{ "operator": "contains", "value": "spotify" }],
            "amount": { "operator": "less_than", "value": "15" },
        },
        "actions": {
            "category_id": category_b,
            "tag_ids": [tag_id],
            "review": { "reviewed": false, "assignee": "anyone" },
        },
    });
    let (status, preview) = call(&app, &cookie, "POST", "/rules/preview", rule.clone()).await;
    assert_eq!(status, 200, "{preview}");
    assert_eq!(preview["match_count"], 1);
    assert_eq!(preview["change_count"], 1);
    assert_eq!(preview["transactions"][0]["before"]["id"], spotify["id"]);
    assert_eq!(
        preview["transactions"][0]["after"]["category_id"],
        category_b
    );
    assert_eq!(preview["transactions"][0]["after"]["reviewed"], false);

    // Previewing stores nothing.
    let uri = format!("/transactions/{}", spotify["id"]);
    let (_, unchanged) = call(&app, &cookie, "GET", &uri, Value::Null).await;
    assert_eq!(unchanged["category_id"], category_a);

    let mut body = rule;
    body["apply_to_existing"] = json!(true);
    let (status, saved) = call(&app, &cookie, "POST", "/rules", body).await;
    assert_eq!(status, 201, "{saved}");
    assert_eq!(saved["last_applied_count"], 1);
    assert!(saved["last_applied_at"].is_string());

    let (_, changed) = call(&app, &cookie, "GET", &uri, Value::Null).await;
    assert_eq!(changed["category_id"], category_b);
    assert_eq!(changed["tags"][0]["id"], tag_id);
    assert_eq!(changed["reviewed"], false);
    let netflix_uri = format!("/transactions/{}", netflix["id"]);
    let (_, untouched) = call(&app, &cookie, "GET", &netflix_uri, Value::Null).await;
    assert_eq!(untouched["category_id"], category_a);

    // Applying again changes nothing more.
    let rule_uri = format!("/rules/{}", saved["id"]);
    let (_, again) = call(
        &app,
        &cookie,
        "PATCH",
        &rule_uri,
        json!({ "apply_to_existing": true }),
    )
    .await;
    assert_eq!(again["last_applied_count"], 0);
}
