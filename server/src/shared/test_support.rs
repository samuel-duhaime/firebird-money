//! Helpers shared by several features' integration tests.

use actix_http::Request;
use actix_web::body::MessageBody;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::test;

/// The id of an account in the caller's household, for tests that just need *some* valid
/// `account_id` to create transactions with. Reuses the household's first account if it already
/// has one, otherwise creates a custom "Test Bank" institution and a "Test Account" in it. The app
/// under test must have `institutions::configure` and `accounts::configure` wired in.
pub async fn test_account_id<S, B>(app: &S, cookie: &str) -> i64
where
    S: Service<Request, Response = ServiceResponse<B>, Error = actix_web::Error>,
    B: MessageBody,
{
    let req = test::TestRequest::get()
        .uri("/accounts")
        .insert_header(("Cookie", cookie))
        .to_request();
    let accounts: serde_json::Value = test::call_and_read_body_json(app, req).await;
    if let Some(id) = accounts
        .as_array()
        .and_then(|a| a.first())
        .and_then(|a| a["id"].as_i64())
    {
        return id;
    }

    let req = test::TestRequest::post()
        .uri("/institutions")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({ "name": "Test Bank" }))
        .to_request();
    let institution: serde_json::Value = test::call_and_read_body_json(app, req).await;
    let institution_id = institution["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created institution, got {institution}"));

    let req = test::TestRequest::post()
        .uri("/accounts")
        .insert_header(("Cookie", cookie))
        .set_json(serde_json::json!({
            "institution_id": institution_id,
            "name": "Test Account",
            "type": "checking",
        }))
        .to_request();
    let account: serde_json::Value = test::call_and_read_body_json(app, req).await;
    account["id"]
        .as_i64()
        .unwrap_or_else(|| panic!("expected created account, got {account}"))
}
