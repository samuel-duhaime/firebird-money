//! Minimal HTTP API: JSON on `GET /`.

use actix_web::{web, App, HttpServer, Responder};
use serde::Serialize;

/// Body returned for `GET /` (`Content-Type: application/json`).
#[derive(Serialize)]
struct HelloResponse {
    message: &'static str,
}

/// Serves `{"message":"hello world"}`.
async fn hello() -> impl Responder {
    web::Json(HelloResponse {
        message: "hello world",
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Host/port the process listens on (change if 3055 is taken).
    let addr = "127.0.0.1:3055";
    println!("Server listening on http://{addr}/");

    HttpServer::new(|| App::new().route("/", web::get().to(hello)))
        .bind(addr)?
        .run()
        .await
}
