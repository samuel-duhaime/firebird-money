//! HTTP API for transactions: JSON CRUD backed by Postgres, plus statement import.

use actix_multipart::form::bytes::Bytes as UploadBytes;
use actix_multipart::form::text::Text;
use actix_multipart::form::{MultipartForm, MultipartFormConfig};
use actix_multipart::MultipartError;
use actix_web::error::{InternalError, PayloadError};
use actix_web::http::StatusCode;
use actix_web::web::Bytes;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::sync::mpsc::UnboundedSender;
use unic_langid::LanguageIdentifier;

use super::download;
use super::import::{self, HouseholdLists, LineCounter, MatchContext, Statement, StatementError};
use super::model::{
    BulkDeleteRequest, BulkUpdateRequest, ImportRequest, NewTransaction, StatementReadEvent,
    Transaction, TransactionFilter, TransactionPatch,
};
use super::repository;
use crate::features::accounts::ACCOUNT_TYPES;
use crate::features::auth::CurrentUser;
use crate::features::{accounts, categories, category_groups, merchants, tags};
use crate::shared::anthropic::{AnthropicClient, AnthropicError};
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_foreign_key_violation,
    is_foreign_key_violation_on, not_found_response,
};
use crate::shared::l10n::L10n;

/// Transaction id path (`/transactions/{id}`)
#[derive(Deserialize)]
struct TransactionIdPath {
    id: u32,
}

/// Multipart body for `POST /transactions/import/read`: the statement, capped at 10 MB and held in
/// memory only (never written to a temp file, unlike a `TempFile` field), plus the account it's
/// from when the user picked one.
#[derive(MultipartForm)]
struct StatementForm {
    #[multipart(limit = "10MB")]
    file: UploadBytes,
    account_id: Option<Text<i32>>,
}

/// Most lines one `POST /transactions/import` accepts — far past any real statement, low enough to
/// keep one request's database transaction short.
const MAX_IMPORT_LINES: usize = 1000;

/// The FK on `transactions (household_id, account_id)` — see the
/// `replace_transactions_account_with_account_id` migration. Every other FK a transaction write can
/// trip is the category one, so this is how a 400 tells the two apart.
const ACCOUNT_FKEY: &str = "transactions_household_account_fkey";

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
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let has_statement = new_transaction
        .original_statement
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());
    if new_transaction.merchant_id.is_none() && !has_statement {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "transaction-merchant-required",
        );
    }

    match repository::create(&pool, household_id, current_user.user.id, &new_transaction).await {
        Ok(transaction) => HttpResponse::Created()
            .insert_header(("Location", format!("/transactions/{}", transaction.id)))
            .json(transaction),
        Err(sqlx::Error::RowNotFound) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "merchant-not-found",
            new_transaction.merchant_id.unwrap_or_default() as u32,
        ),
        Err(e) if is_foreign_key_violation_on(&e, ACCOUNT_FKEY) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "account-not-found",
            new_transaction.account_id as u32,
        ),
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

/// `GET /transactions` — list transactions, optionally filtered by `date`, a `start_date`/
/// `end_date` range, `merchant_id`, `account_id`, a free-text `search` matched against merchant
/// name, original statement, category, and amount, comma-separated `category_ids`/`merchant_ids`/
/// `account_ids`/`tag_ids` (plus `untagged=true`), and/or `amount_op` (`greater_than`,
/// `less_than`, `equal_to`, `between`) with `amount` (and `amount_max` for `between`). Accepts
/// `order` (`date`, `inverse_date`, `amount`, `inverse_amount`) to control sort order.
async fn list_transactions(
    filter: web::Query<TransactionFilter>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::list(&pool, household_id, &filter).await {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => {
            error!("failed to list transactions error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /transactions/download` — download the same (filtered/sorted) transactions as
/// `GET /transactions`, rendered as a CSV or Excel file. Accepts the same filter and `order` query
/// params, plus `format` (`csv` or `xlsx`).
async fn download_transactions(
    filter: web::Query<TransactionFilter>,
    format: web::Query<DownloadFormatQuery>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let transactions = match repository::list(&pool, household_id, &filter).await {
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

/// Converts a multipart parsing/extraction failure — in practice almost always the 10 MB field
/// size limit on `StatementForm` being exceeded — into the same localized JSON error shape the
/// rest of this API uses, instead of actix-multipart's plain-text default.
fn import_upload_error_handler(err: MultipartError, req: &HttpRequest) -> actix_web::Error {
    let response = match req.app_data::<web::Data<L10n>>() {
        Some(l10n) => {
            let locale = l10n.locale();
            let message_id = match &err {
                // The field-level `#[multipart(limit = "10MB")]` surfaces as a payload overflow,
                // not `MultipartError::Field` — confirmed against a real oversized upload.
                MultipartError::Payload(PayloadError::Overflow) | MultipartError::Field { .. } => {
                    "import-file-too-large"
                }
                _ => "import-file-required",
            };
            error_response(l10n, &locale, StatusCode::BAD_REQUEST, message_id)
        }
        None => HttpResponse::BadRequest().finish(),
    };
    InternalError::from_response(err, response).into()
}

/// Loads everything statement import shows Claude: the household's accounts, categories (with
/// their group's name and type), tags, and active merchants.
async fn load_household_lists(
    pool: &PgPool,
    household_id: i32,
) -> Result<HouseholdLists, sqlx::Error> {
    let groups = category_groups::repository::list(pool, household_id).await?;
    let categories = categories::repository::list(pool, household_id)
        .await?
        .into_iter()
        .filter_map(|category| {
            let group = groups.iter().find(|g| g.id == category.group_id)?;
            Some(import::PromptCategory {
                id: category.id,
                name_en: category.name_en,
                name_fr: category.name_fr,
                group: group.name_en.clone(),
                kind: group.r#type.clone(),
            })
        })
        .collect();
    let accounts = accounts::repository::list(pool, household_id)
        .await?
        .into_iter()
        .map(|account| import::PromptAccount {
            id: account.id,
            name: account.name,
            institution: account.institution_name,
            kind: account.r#type,
            last_four: account.last_four,
        })
        .collect();
    let tags = tags::repository::list(pool, household_id)
        .await?
        .into_iter()
        .map(|tag| import::PromptNamed {
            id: tag.id,
            name: tag.name,
        })
        .collect();
    let merchants = merchants::repository::list_active(pool, household_id)
        .await?
        .into_iter()
        .map(|(id, name)| import::PromptNamed { id, name })
        .collect();

    Ok(HouseholdLists {
        accounts,
        categories,
        tags,
        merchants,
    })
}

/// `POST /transactions/import/read` — reads a bank statement (multipart: `file`, up to 10 MB — a
/// PDF, image, CSV, or Excel/OpenDocument spreadsheet — and optional `account_id`) with Claude and streams back its lines, matched to the
/// caller's household, as newline-delimited `StatementReadEvent`s. Stores nothing: the file is
/// only held in memory for the one Claude call, and the lines only come back to the caller, who
/// sends the accepted ones to `POST /transactions/import`.
///
/// Everything that can be checked up front (household, Claude configured, file type, account) is
/// answered as an ordinary JSON error before the stream starts; once it has, a failure arrives as
/// a final `error` event instead.
async fn read_statement(
    MultipartForm(form): MultipartForm<StatementForm>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
    anthropic: web::Data<AnthropicClient>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    if !anthropic.is_configured() {
        return error_response(
            &l10n,
            &locale,
            StatusCode::SERVICE_UNAVAILABLE,
            "import-unavailable",
        );
    }
    // Off the async workers: converting a spreadsheet is CPU-bound.
    let upload = form.file.data.clone();
    let statement = match web::block(move || Statement::from_upload(upload)).await {
        Ok(Ok(statement)) => statement,
        Ok(Err(StatementError::Unsupported)) => {
            return error_response(
                &l10n,
                &locale,
                StatusCode::BAD_REQUEST,
                "import-file-unsupported",
            )
        }
        Ok(Err(StatementError::TooLong)) => {
            return error_response(
                &l10n,
                &locale,
                StatusCode::BAD_REQUEST,
                "import-file-too-long",
            )
        }
        Err(e) => {
            error!("statement import failed to prepare the file error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };

    let loaded = match load_household_lists(&pool, household_id).await {
        Ok(lists) => merchants::repository::match_candidates(&pool, household_id)
            .await
            .map(|candidates| (lists, candidates)),
        Err(e) => Err(e),
    };
    let (lists, merchant_candidates) = match loaded {
        Ok(loaded) => loaded,
        Err(e) => {
            error!("failed to load household lists for statement import error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };

    let selected_account_id = form.account_id.map(Text::into_inner);
    if let Some(id) = selected_account_id {
        if !lists.accounts.iter().any(|account| account.id == id) {
            return error_response_with_n(
                &l10n,
                &locale,
                StatusCode::BAD_REQUEST,
                "account-not-found",
                id as u32,
            );
        }
    }

    let request = import::build_request(&statement, &lists, selected_account_id);
    // From here on the statement only lives inside `request`, dropped once Claude has answered.
    drop(statement);
    drop(form.file);

    let (events, received) = tokio::sync::mpsc::unbounded_channel();
    actix_web::rt::spawn(stream_statement_read(
        StatementRead {
            anthropic: anthropic.into_inner(),
            pool: pool.into_inner(),
            l10n: l10n.into_inner(),
            locale,
            household_id,
            lists,
            merchant_candidates,
            selected_account_id,
        },
        request,
        events,
    ));

    let body = futures_util::stream::unfold(received, |mut received| async move {
        let event = received.recv().await?;
        let mut line = serde_json::to_vec(&event).expect("events always serialize");
        line.push(b'\n');
        Some((Ok::<_, actix_web::Error>(Bytes::from(line)), received))
    });
    HttpResponse::Ok()
        .content_type("application/x-ndjson")
        // Keeps proxies from holding the stream back until it ends.
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(body)
}

/// Everything the background half of `read_statement` needs once the response has started.
struct StatementRead {
    anthropic: std::sync::Arc<AnthropicClient>,
    pool: std::sync::Arc<PgPool>,
    l10n: std::sync::Arc<L10n>,
    locale: LanguageIdentifier,
    household_id: i32,
    lists: HouseholdLists,
    merchant_candidates: Vec<(i32, String)>,
    selected_account_id: Option<i32>,
}

/// Reads the statement and sends each step to `events`. Stops early, abandoning the Claude call,
/// as soon as nobody is listening anymore (the client went away), so a dropped request doesn't
/// keep spending tokens.
async fn stream_statement_read(
    read: StatementRead,
    request: serde_json::Value,
    events: UnboundedSender<StatementReadEvent>,
) {
    let fail = |message_id: &str| {
        let error = read.l10n.format_message(&read.locale, message_id, None);
        let _ = events.send(StatementReadEvent::Error { error });
    };

    if events.send(StatementReadEvent::Reading).is_err() {
        return;
    }

    let mut counter = LineCounter::default();
    let answer = read
        .anthropic
        .stream_message(request, |text| match counter.update(text) {
            Some(count) => events.send(StatementReadEvent::Found { count }).is_ok(),
            None => !events.is_closed(),
        })
        .await;
    let lines = match answer.map(|text| import::parse_answer(&text)) {
        Ok(Ok(lines)) => lines,
        Err(AnthropicError::Cancelled) => return,
        Err(AnthropicError::Truncated) => return fail("import-file-too-long"),
        Err(e @ (AnthropicError::Refusal | AnthropicError::UnexpectedResponse)) => {
            error!("statement import could not read the file error={e}");
            return fail("import-file-unreadable");
        }
        Ok(Err(e)) => {
            // Only the error's kind and position: its message can quote statement text.
            error!(
                "statement import got an answer that isn't the expected JSON kind={:?} line={} column={}",
                e.classify(),
                e.line(),
                e.column()
            );
            return fail("import-file-unreadable");
        }
        Err(e) => {
            error!("statement import Claude call failed error={e}");
            return fail("import-read-failed");
        }
    };

    if events.send(StatementReadEvent::Matching).is_err() {
        return;
    }
    let mut transactions = import::resolve(
        lines,
        &MatchContext {
            lists: &read.lists,
            merchant_candidates: &read.merchant_candidates,
            selected_account_id: read.selected_account_id,
        },
    );

    if events.send(StatementReadEvent::CheckingDuplicates).is_err() {
        return;
    }
    let mut dates: Vec<_> = transactions.iter().map(|t| t.date).collect();
    dates.sort_unstable();
    dates.dedup();
    match repository::list_on_dates(&read.pool, read.household_id, &dates).await {
        Ok(existing) => import::flag_duplicates(&mut transactions, &existing),
        Err(e) => {
            error!("statement import failed to look for duplicates error={e}");
            return fail("internal-db-error");
        }
    }

    let _ = events.send(StatementReadEvent::Done { transactions });
}

/// `POST /transactions/import` — adds the statement lines the user accepted (`ImportRequest`), all
/// or nothing, each stored with `source: import`. Creates any `new_merchant_name` that doesn't
/// exist yet. Answers `201` with how many transactions and merchants were created.
async fn import_transactions(
    body: web::Json<ImportRequest>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let lines = &body.transactions;
    if lines.is_empty() {
        return error_response(&l10n, &locale, StatusCode::BAD_REQUEST, "import-empty");
    }
    if lines.len() > MAX_IMPORT_LINES {
        return error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "import-too-many",
            MAX_IMPORT_LINES as u32,
        );
    }
    let each_has_one_merchant = lines.iter().all(|line| {
        let has_new_name = line
            .new_merchant_name
            .as_deref()
            .is_some_and(|name| !name.trim().is_empty());
        line.merchant_id.is_some() != has_new_name
    });
    let each_has_one_account = lines.iter().all(|line| {
        let has_new_account = line.new_account.as_ref().is_some_and(|account| {
            !account.institution_name.trim().is_empty()
                && !account.name.trim().is_empty()
                && ACCOUNT_TYPES.contains(&account.kind.as_str())
                // Same rule as the `accounts_last_four_valid` CHECK.
                && account.last_four.as_deref().is_none_or(|digits| {
                    (1..=4).contains(&digits.len())
                        && digits.chars().all(|c| c.is_ascii_alphanumeric())
                })
        });
        line.account_id.is_some() != has_new_account
    });
    if !each_has_one_account {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "import-account-required",
        );
    }
    if !each_has_one_merchant {
        return error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "import-merchant-required",
        );
    }

    match repository::create_imported(&pool, household_id, current_user.user.id, lines).await {
        Ok(result) => HttpResponse::Created().json(result),
        Err(sqlx::Error::RowNotFound) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "import-merchant-not-found",
        ),
        Err(e) if is_foreign_key_violation_on(&e, ACCOUNT_FKEY) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "import-account-not-found",
        ),
        Err(e) if is_foreign_key_violation(&e) => error_response(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "import-category-not-found",
        ),
        Err(e) => {
            error!("failed to import transactions error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /transactions/{id}` — fetch a single transaction.
async fn get_transaction(
    path: web::Path<TransactionIdPath>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::get(&pool, household_id, i64::from(id)).await {
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
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::update(&pool, household_id, i64::from(id), &patch).await {
        Ok(Some(transaction)) => HttpResponse::Ok().json(transaction),
        Ok(None) => not_found_response(&l10n, &locale, "transaction-not-found", id),
        Err(sqlx::Error::RowNotFound) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "merchant-not-found",
            patch.merchant_id.unwrap_or_default() as u32,
        ),
        Err(e) if is_foreign_key_violation_on(&e, ACCOUNT_FKEY) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "account-not-found",
            patch.account_id.unwrap_or_default() as u32,
        ),
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
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let id = path.id;
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::delete(&pool, household_id, i64::from(id)).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "transaction-not-found", id),
        Err(e) => {
            error!("failed to delete transaction id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /transactions/bulk` — partially update several transactions at once (only `Some` fields
/// change, on every id); unset fields are left unchanged. Ids the household doesn't own are
/// silently skipped rather than erroring the whole batch.
async fn bulk_update_transactions(
    body: web::Json<BulkUpdateRequest>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    if body.ids.is_empty() {
        return HttpResponse::Ok().json(Vec::<Transaction>::new());
    }

    match repository::bulk_update(&pool, household_id, &body.ids, &body.patch).await {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(sqlx::Error::RowNotFound) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "merchant-not-found",
            body.patch.merchant_id.unwrap_or_default() as u32,
        ),
        Err(e) if is_foreign_key_violation_on(&e, ACCOUNT_FKEY) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "account-not-found",
            body.patch.account_id.unwrap_or_default() as u32,
        ),
        Err(e) if is_foreign_key_violation(&e) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "category-not-found",
            body.patch.category_id.unwrap_or_default() as u32,
        ),
        Err(e) => {
            error!("failed to bulk update transactions error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /transactions/bulk` — delete several transactions at once. Ids the household doesn't
/// own are silently skipped rather than erroring the whole batch.
async fn bulk_delete_transactions(
    body: web::Json<BulkDeleteRequest>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    if body.ids.is_empty() {
        return HttpResponse::NoContent().finish();
    }

    match repository::bulk_delete(&pool, household_id, &body.ids).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            error!("failed to bulk delete transactions error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the transactions feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.app_data(MultipartFormConfig::default().error_handler(import_upload_error_handler))
        .route("/transactions", web::get().to(list_transactions))
        .route("/transactions", web::post().to(create_transaction))
        .route(
            "/transactions/download",
            web::get().to(download_transactions),
        )
        .route("/transactions/import/read", web::post().to(read_statement))
        .route("/transactions/import", web::post().to(import_transactions))
        .route(
            "/transactions/bulk",
            web::patch().to(bulk_update_transactions),
        )
        .route(
            "/transactions/bulk",
            web::delete().to(bulk_delete_transactions),
        )
        .route("/transactions/{id}", web::get().to(get_transaction))
        .route("/transactions/{id}", web::patch().to(update_transaction))
        .route("/transactions/{id}", web::delete().to(delete_transaction));
}
