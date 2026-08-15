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
}

/// Creates a household through `POST /households` and returns its id, for tests that only need an
/// existing row to act on.
async fn create_via_api<S, B>(app: &S) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::post().uri("/households").to_request();
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
    let req = test::TestRequest::post().uri("/households").to_request();
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

// --- GET /households/{id} ---

#[sqlx::test]
async fn get_household_returns_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app).await;

    let req = test::TestRequest::get()
        .uri(&format!("/households/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["id"].as_i64().unwrap(), id);
}

#[sqlx::test]
async fn get_household_omits_join_code(pool: PgPool) {
    // `join_code` is a shared secret; this route has no auth check yet (that's #65), so it must
    // not hand the code to just anyone who can guess an id.
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app).await;

    let req = test::TestRequest::get()
        .uri(&format!("/households/{id}"))
        .to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert!(body.get("join_code").is_none());
}

#[sqlx::test]
async fn create_household_returns_its_join_code(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::post().uri("/households").to_request();
    let body: serde_json::Value = test::call_and_read_body_json(&app, req).await;

    assert_eq!(body["join_code"].as_str().unwrap().len(), 8);
}

#[sqlx::test]
async fn get_household_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::get()
        .uri("/households/999999")
        .to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 404);
    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].is_string());
}

// --- DELETE /households/{id} ---

#[sqlx::test]
async fn delete_household_removes_row(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let id = create_via_api(&app).await;

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/households/{id}"))
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;
    assert_eq!(delete_resp.status(), 204);

    let get_req = test::TestRequest::get()
        .uri(&format!("/households/{id}"))
        .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[sqlx::test]
async fn delete_household_not_found(pool: PgPool) {
    let app = test::init_service(app_with(pool)).await;
    let req = test::TestRequest::delete()
        .uri("/households/999999")
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
            .configure(configure)
            .configure(crate::features::users::configure)
            .configure(crate::features::household_members::configure),
    )
    .await;
    let household_id = create_via_api(&app).await;

    let user_req = test::TestRequest::post()
        .uri("/users")
        .set_json(serde_json::json!({ "email": "member@example.com" }))
        .to_request();
    let user_resp = test::call_service(&app, user_req).await;
    let user_body: serde_json::Value = test::read_body_json(user_resp).await;
    let user_id = user_body["id"].as_i64().unwrap();

    let member_req = test::TestRequest::post()
        .uri("/household-members")
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
        .to_request();
    let delete_resp = test::call_service(&app, delete_req).await;

    assert_eq!(delete_resp.status(), 409);
    let body: serde_json::Value = test::read_body_json(delete_resp).await;
    assert!(body["error"].is_string());
}
