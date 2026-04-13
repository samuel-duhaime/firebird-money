//! HTTP API: sample CSV on `GET /transactions/sample`, export CSV on
//! `GET /transactions/{transaction_number}`.

mod l10n;

use std::io::ErrorKind;
use std::path::PathBuf;

use actix_web::http::header::{ContentLanguage, LanguageTag, QualityItem};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use log::error;
use serde::Deserialize;

use crate::l10n::L10n;

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
async fn transactions_by_number(
    path: web::Path<TransactionNumberPath>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let lang_tag = locale.to_string();

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
        Ok(Err(e)) if e.kind() == ErrorKind::NotFound => {
            let body = l10n.format_with_n(&locale, "file-not-found", n);
            HttpResponse::NotFound()
                .insert_header(ContentLanguage(vec![QualityItem::max(
                    LanguageTag::parse(&lang_tag).unwrap_or_else(|_| LanguageTag::parse("en").unwrap()),
                )]))
                .content_type("text/plain; charset=utf-8")
                .body(body)
        }
        // If the file is found but there is an error reading it, return a 500 error
        Ok(Err(e)) => {
            error!(
                "failed to read transactions CSV transaction_number={n} path={path_for_log} error={e}"
            );
            let body = l10n.format_message(&locale, "internal-read-error", None);
            HttpResponse::InternalServerError()
                .insert_header(ContentLanguage(vec![QualityItem::max(
                    LanguageTag::parse(&lang_tag).unwrap_or_else(|_| LanguageTag::parse("en").unwrap()),
                )]))
                .content_type("text/plain; charset=utf-8")
                .body(body)
        }
        // If there is an error reading the file, return a 500 error
        Err(e) => {
            error!(
                "blocking read task failed transaction_number={n} path={path_for_log} error={e}"
            );
            let body = l10n.format_message(&locale, "internal-read-error", None);
            HttpResponse::InternalServerError()
                .insert_header(ContentLanguage(vec![QualityItem::max(
                    LanguageTag::parse(&lang_tag).unwrap_or_else(|_| LanguageTag::parse("en").unwrap()),
                )]))
                .content_type("text/plain; charset=utf-8")
                .body(body)
        }
    }
}

/// Main function to start the server
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let addr = "127.0.0.1:3055";
    println!("Server listening on http://{addr}/transactions/sample");
    println!("  and http://{addr}/transactions/{{n}}  (e.g. /transactions/1)");

    let l10n = web::Data::new(L10n::new());

    // Actix web server configuration
    HttpServer::new(move || {
        App::new()
            .app_data(l10n.clone())
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
