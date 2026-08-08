//! Integration tests for the household members HTTP API.
//!
//! Each `#[sqlx::test]` gets its own throwaway Postgres database (migrated from
//! `migrations/`, dropped afterwards), so these never touch real dev data.

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::{test, web, App};
use sqlx::PgPool;

use super::handlers::configure;
use crate::shared::l10n::L10n;

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
        .configure(configure)
        .configure(crate::features::households::configure)
        .configure(crate::features::users::configure)
}

async fn create_household<S, B>(app: &S) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post().uri("/households").to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"].as_i64().unwrap()
}

async fn create_user<S, B>(app: &S, email: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/users")
        .set_json(serde_json::json!({ "email": email }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"].as_i64().unwrap()
}

async fn create_member_via_api<S, B>(
    app: &S,
    household_id: i64,
    user_id: i64,
    member_type: &str,
) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post()
        .uri("/household-members")
        .set_json(serde_json::json!({
            "household_id": household_id,
            "user_id": user_id,
            "type": member_type,
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    body["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created household member, got {body}"))
}

// --- POST /household-members ---

#[sqlx::test]
async fn create_household_member_returns_created_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_id = create_household(&app).await;
    let user_id = create_user(&app, "jane@example.com").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .set_json(serde_json::json!({
            "household_id": household_id,
            "user_id": user_id,
            "type": "family_manager",
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
    assert_eq!(body["household_id"].as_i64().unwrap(), household_id);
    assert_eq!(body["user_id"].as_i64().unwrap(), user_id);
    assert_eq!(body["type"], "family_manager");
    assert_eq!(
        location,
        format!("/household-members/{}", body["id"].as_i64().unwrap())
    );
}

#[sqlx::test]
async fn create_household_member_rejects_invalid_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_id = create_household(&app).await;
    let user_id = create_user(&app, "jane@example.com").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .set_json(serde_json::json!({
            "household_id": household_id,
            "user_id": user_id,
            "type": "nonsense",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_household_member_rejects_unknown_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let user_id = create_user(&app, "jane@example.com").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .set_json(serde_json::json!({
            "household_id": 999999,
            "user_id": user_id,
            "type": "family_manager",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

#[sqlx::test]
async fn create_household_member_rejects_unknown_user(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_id = create_household(&app).await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .set_json(serde_json::json!({
            "household_id": household_id,
            "user_id": 999999,
            "type": "family_manager",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

#[sqlx::test]
async fn create_household_member_rejects_duplicate_pair(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_id = create_household(&app).await;
    let user_id = create_user(&app, "jane@example.com").await;
    create_member_via_api(&app, household_id, user_id, "family_manager").await;

    let req = test::TestRequest::post()
        .uri("/household-members")
        .set_json(serde_json::json!({
            "household_id": household_id,
            "user_id": user_id,
            "type": "family_member",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 409);
}

// --- GET /household-members ---

#[sqlx::test]
async fn list_household_members_filters_by_household(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_a = create_household(&app).await;
    let household_b = create_household(&app).await;
    let user_1 = create_user(&app, "one@example.com").await;
    let user_2 = create_user(&app, "two@example.com").await;
    create_member_via_api(&app, household_a, user_1, "family_manager").await;
    create_member_via_api(&app, household_b, user_2, "family_manager").await;

    let req = test::TestRequest::get()
        .uri(&format!("/household-members?household_id={household_a}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["household_id"].as_i64().unwrap(), household_a);
}

#[sqlx::test]
async fn list_household_members_filters_by_user(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_a = create_household(&app).await;
    let household_b = create_household(&app).await;
    let user_id = create_user(&app, "jane@example.com").await;
    create_member_via_api(&app, household_a, user_id, "family_manager").await;
    create_member_via_api(&app, household_b, user_id, "family_member").await;

    let req = test::TestRequest::get()
        .uri(&format!("/household-members?user_id={user_id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;
    let rows = body.as_array().unwrap();

    assert_eq!(rows.len(), 2);
}

// --- GET /household-members/{id} ---

#[sqlx::test]
async fn get_household_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get()
        .uri("/household-members/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}

// --- PATCH /household-members/{id} ---

#[sqlx::test]
async fn update_household_member_changes_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_id = create_household(&app).await;
    let user_id = create_user(&app, "jane@example.com").await;
    let id = create_member_via_api(&app, household_id, user_id, "family_member").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{id}"))
        .set_json(serde_json::json!({ "type": "family_manager" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["type"], "family_manager");
}

#[sqlx::test]
async fn update_household_member_rejects_invalid_type(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_id = create_household(&app).await;
    let user_id = create_user(&app, "jane@example.com").await;
    let id = create_member_via_api(&app, household_id, user_id, "family_member").await;

    let req = test::TestRequest::patch()
        .uri(&format!("/household-members/{id}"))
        .set_json(serde_json::json!({ "type": "nonsense" }))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 400);
}

// --- DELETE /household-members/{id} ---

#[sqlx::test]
async fn delete_household_member_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let household_id = create_household(&app).await;
    let user_id = create_user(&app, "jane@example.com").await;
    let id = create_member_via_api(&app, household_id, user_id, "family_member").await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/household-members/{id}"))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/household-members/{id}"))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_household_member_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::delete()
        .uri("/household-members/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
}
