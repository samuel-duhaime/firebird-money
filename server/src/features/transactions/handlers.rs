//! HTTP API for transactions: JSON CRUD backed by Postgres, plus the async budget-file import.

use actix_multipart::form::tempfile::TempFile;
use actix_multipart::form::MultipartForm;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use super::download;
use super::import;
use super::jobs::JobStore;
use super::model::{ImportJobReport, NewTransaction, TransactionFilter, TransactionPatch};
use super::repository;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_foreign_key_violation,
    not_found_response,
};
use crate::shared::l10n::L10n;

/// Transaction id path (`/transactions/{id}`)
#[derive(Deserialize)]
struct TransactionIdPath {
    id: u32,
}

/// Import job id path (`/transactions/import/jobs/{id}`)
#[derive(Deserialize)]
struct ImportJobIdPath {
    id: Uuid,
}

/// Multipart body for `POST /transactions/import`: a single file field, capped at 10 MB.
#[derive(MultipartForm)]
struct ImportUploadForm {
    #[multipart(limit = "10MB")]
    file: TempFile,
}

/// File format for `GET /transactions/download`.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DownloadFormat {
    Csv,
    Xlsx,
}

/// Query params for `GET /transactions/download`, parsed independently from `TransactionFilter`
/// since both are read from the same query string.
#[derive(Deserialize)]
struct DownloadFormatQuery {
    format: DownloadFormat,
}

/// `POST /transactions` — create a transaction.
async fn create_transaction(
    new_transaction: web::Json<NewTransaction>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    match repository::create(&pool, &new_transaction).await {
        Ok(transaction) => HttpResponse::Created()
            .insert_header(("Location", format!("/transactions/{}", transaction.id)))
            .json(transaction),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-not-found",
            new_transaction.category_id as u32,
        ),
        Err(e) => {
            error!("failed to create transaction error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /transactions` — list transactions, optionally filtered by `date`, `merchant`, and/or a
/// free-text `search` matched against merchant, category, and amount. Accepts `order` (`date`,
/// `inverse_date`, `amount`, `inverse_amount`) to control sort order.
async fn list_transactions(
    filter: web::Query<TransactionFilter>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    match repository::list(&pool, &filter).await {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => {
            error!("failed to list transactions error={e}");
            internal_error_response(&l10n, &l10n.locale())
        }
    }
}

/// `GET /transactions/download` — download the same (filtered/sorted) transactions as
/// `GET /transactions`, rendered as a CSV or Excel file. Accepts the same `date`, `merchant`,
/// `search`, and `order` query params, plus `format` (`csv` or `xlsx`).
async fn download_transactions(
    filter: web::Query<TransactionFilter>,
    format: web::Query<DownloadFormatQuery>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();

    let transactions = match repository::list(&pool, &filter).await {
        Ok(transactions) => transactions,
        Err(e) => {
            error!("failed to list transactions for download error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };

    match format.format {
        DownloadFormat::Csv => match download::to_csv(&transactions) {
            Ok(bytes) => {
                let filename = download::filename(&filter, "csv");
                HttpResponse::Ok()
                    .content_type("text/csv; charset=utf-8")
                    .insert_header((
                        "Content-Disposition",
                        format!("attachment; filename=\"{filename}\""),
                    ))
                    .body(bytes)
            }
            Err(e) => {
                error!("failed to render transactions as csv error={e}");
                internal_error_response(&l10n, &locale)
            }
        },
        DownloadFormat::Xlsx => match download::to_xlsx(&transactions) {
            Ok(bytes) => {
                let filename = download::filename(&filter, "xlsx");
                HttpResponse::Ok()
                    .content_type(
                        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                    )
                    .insert_header((
                        "Content-Disposition",
                        format!("attachment; filename=\"{filename}\""),
                    ))
                    .body(bytes)
            }
            Err(e) => {
                error!("failed to render transactions as xlsx error={e}");
                internal_error_response(&l10n, &locale)
            }
        },
    }
}

/// `POST /transactions/import` — accepts a single budget-export file, stages it to disk, and
/// spawns an unattended `claude` subprocess (running the `budget-file-to-transaction` skill) to
/// parse and import it in the background. Returns immediately with the job id; poll
/// `GET /transactions/import/jobs/{id}` for status.
async fn import_transactions(
    MultipartForm(form): MultipartForm<ImportUploadForm>,
    job_store: web::Data<JobStore>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();

    let original_name = form.file.file_name.clone().unwrap_or_default();
    if form.file.size == 0 || original_name.trim().is_empty() {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "import-file-required",
        );
    }

    let job = job_store.create(import::sanitize_filename(&original_name));
    let dest_dir = import::upload_dir(job.id);
    let dest_path = dest_dir.join(import::sanitize_filename(&original_name));

    if let Err(e) = tokio::fs::create_dir_all(&dest_dir).await {
        error!(
            "failed to create import upload dir job_id={} error={e}",
            job.id
        );
        return internal_error_response(&l10n, &locale);
    }
    if let Err(e) = tokio::fs::copy(form.file.file.path(), &dest_path).await {
        error!("failed to stage import upload job_id={} error={e}", job.id);
        return internal_error_response(&l10n, &locale);
    }

    let job_store_for_task = job_store.clone();
    let job_id = job.id;
    tokio::spawn(async move {
        import::run_import(&job_store_for_task, job_id, dest_path).await;
    });

    HttpResponse::Accepted()
        .insert_header(("Location", format!("/transactions/import/jobs/{}", job.id)))
        .json(job)
}

/// `GET /transactions/import/jobs/{id}` — poll the status of an import job.
async fn get_import_job(
    path: web::Path<ImportJobIdPath>,
    job_store: web::Data<JobStore>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    match job_store.get(path.id) {
        Some(job) => HttpResponse::Ok().json(job),
        None => error_response(
            &l10n,
            &l10n.locale(),
            StatusCode::NOT_FOUND,
            "import-job-not-found",
        ),
    }
}

/// `PATCH /transactions/import/jobs/{id}` — how the unattended import subprocess reports its own
/// final result back to the server (see the skill's "Unattended mode" section). Not intended to
/// be called from the client.
async fn report_import_job(
    path: web::Path<ImportJobIdPath>,
    report: web::Json<ImportJobReport>,
    job_store: web::Data<JobStore>,
) -> impl Responder {
    job_store.complete(
        path.id,
        report.status,
        report.created_count,
        report.failed_count,
        report.skipped_count,
        report.error_message.clone(),
    );
    HttpResponse::NoContent().finish()
}

/// `GET /transactions/{id}` — fetch a single transaction.
async fn get_transaction(
    path: web::Path<TransactionIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::get(&pool, i64::from(id)).await {
        Ok(Some(transaction)) => HttpResponse::Ok().json(transaction),
        Ok(None) => not_found_response(&l10n, &locale, "transaction-not-found", id),
        Err(e) => {
            error!("failed to get transaction id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /transactions/{id}` — partially update a transaction; unset fields are left unchanged.
async fn update_transaction(
    path: web::Path<TransactionIdPath>,
    patch: web::Json<TransactionPatch>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::update(&pool, i64::from(id), &patch).await {
        Ok(Some(transaction)) => HttpResponse::Ok().json(transaction),
        Ok(None) => not_found_response(&l10n, &locale, "transaction-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-not-found",
            patch.category_id.unwrap_or_default() as u32,
        ),
        Err(e) => {
            error!("failed to update transaction id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /transactions/{id}` — delete a transaction.
async fn delete_transaction(
    path: web::Path<TransactionIdPath>,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;

    match repository::delete(&pool, i64::from(id)).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "transaction-not-found", id),
        Err(e) => {
            error!("failed to delete transaction id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the transactions feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/transactions", web::get().to(list_transactions))
        .route("/transactions", web::post().to(create_transaction))
        .route(
            "/transactions/download",
            web::get().to(download_transactions),
        )
        .route("/transactions/import", web::post().to(import_transactions))
        .route(
            "/transactions/import/jobs/{id}",
            web::get().to(get_import_job),
        )
        .route(
            "/transactions/import/jobs/{id}",
            web::patch().to(report_import_job),
        )
        .route("/transactions/{id}", web::get().to(get_transaction))
        .route("/transactions/{id}", web::patch().to(update_transaction))
        .route("/transactions/{id}", web::delete().to(delete_transaction));
}
