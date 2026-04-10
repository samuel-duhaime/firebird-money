//! Minimal HTTP API: sample transactions JSON on `GET /transactions/sample`.

use actix_web::{web, App, HttpServer, Responder};
use serde::Serialize;

/// One row; JSON keys match spreadsheet/CSV headers for easy export or hand-editing.
#[derive(Serialize)]
struct Transaction {
    #[serde(rename = "Date")]
    date: String,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Amount")]
    amount: f64,
    #[serde(rename = "Category")]
    category: String,
}

/// Sample transactions response
#[derive(Serialize)]
struct SampleTransactionsResponse {
    transactions: Vec<Transaction>,
}

/// Sample transactions for development (English JSON keys, CSV-friendly).
fn sample_transactions() -> Vec<Transaction> {
    vec![
        Transaction {
            date: "2024-01-15".into(),
            description: "STARBUCKS STORE #4521".into(),
            amount: 12.34,
            category: "Restaurant".into(),
        },
        Transaction {
            date: "2024-01-16".into(),
            description: "IGA SUPERMARKT".into(),
            amount: 56.78,
            category: "Grocery".into(),
        },
    ]
}

/// Sample transactions for development (English JSON keys, CSV-friendly).
async fn transactions_sample() -> impl Responder {
    web::Json(SampleTransactionsResponse {
        transactions: sample_transactions(),
    })
}

/// Main function
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:3055";
    println!("Server listening on http://{addr}/transactions/sample");

    HttpServer::new(|| {
        App::new().route(
            "/transactions/sample",
            web::get().to(transactions_sample),
        )
    })
        .bind(addr)?
        .run()
        .await
}
