//! HTTP API: sample CSV on `GET /transactions/sample`, export CSV on
//! `GET /transactions/{transaction_number}`.

use std::io::ErrorKind;
use std::path::PathBuf;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use log::error;
use serde::Deserialize;

const INTERNAL_READ_ERR: &str = "Internal server error reading file";
const SAMPLE_TRANSACTIONS_CSV: &str = include_str!("../data/transactions/outputs/transactions_sample.csv");

/// Sample transactions CSV
async fn transactions_sample() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/csv; charset=utf-8")
        .body(SAMPLE_TRANSACTIONS_CSV)
}

/// Transaction number path
#[derive(Deserialize)]
struct TransactionNumberPath {
    transaction_number: u32,
}

/// Serves `data/transactions/outputs/transactions_{n}.csv` from disk (relative to crate root).
async fn transactions_by_number(path: web::Path<TransactionNumberPath>) -> impl Responder {
    let n = path.transaction_number;
    let filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/transactions/outputs")
        .join(format!("transactions_{n}.csv"));
    let path_for_log = filepath.display().to_string();

    // Read the transactions CSV file
    match web::block(move || std::fs::read_to_string(&filepath)).await {
        // If the file is found, return the content
        Ok(Ok(body)) => HttpResponse::Ok()
            .content_type("text/csv; charset=utf-8")
            .body(body),
        // If the file is not found, return a 404 error
        Ok(Err(e)) if e.kind() == ErrorKind::NotFound => HttpResponse::NotFound()
            .content_type("text/plain; charset=utf-8")
            .body(format!("No file transactions_{n}.csv under data/transactions/outputs/")),
        // If the file is found but there is an error reading it, return a 500 error
        Ok(Err(e)) => {
            error!(
                "failed to read transactions CSV transaction_number={n} path={path_for_log} error={e}"
            );
            HttpResponse::InternalServerError()
                .content_type("text/plain; charset=utf-8")
                .body(INTERNAL_READ_ERR)
        }
        // If there is an error reading the file, return a 500 error
        Err(e) => {
            error!(
                "blocking read task failed transaction_number={n} path={path_for_log} error={e}"
            );
            HttpResponse::InternalServerError()
                .content_type("text/plain; charset=utf-8")
                .body(INTERNAL_READ_ERR)
        }
    }
}

/// Main function to start the server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let addr = "127.0.0.1:3055";
    println!("Server listening on http://{addr}/transactions/sample");
    println!("  and http://{addr}/transactions/{{n}}  (e.g. /transactions/1)");

    // Actix web server configuration
    HttpServer::new(|| {
        App::new()
            .route("/transactions/sample", web::get().to(transactions_sample))
            .route(
                "/transactions/{transaction_number}",
                web::get().to(transactions_by_number),
            )
    })
        .bind(addr)?
        .run()
        .await
}
