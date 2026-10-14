//! HTTP API for accounts: JSON CRUD backed by Postgres, scoped to the caller's household.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::model::{AccountPatch, NewAccount, ACCOUNT_TYPES};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::shared::http_error::{
    error_response, error_response_with_n, internal_error_response, is_foreign_key_violation,
    not_found_response,
};
use crate::shared::l10n::L10n;

/// Account id path (`/accounts/{id}`)
#[derive(Deserialize)]
struct AccountIdPath {
    id: u32,
}

/// Trims a submitted name; `Err` with the message id to answer with if it's blank.
fn validate_name(name: &str) -> Result<String, &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        Err("account-name-required")
    } else {
        Ok(trimmed.to_string())
    }
}

fn validate_type(r#type: &str) -> Result<String, &'static str> {
    if ACCOUNT_TYPES.contains(&r#type) {
        Ok(r#type.to_string())
    } else {
        Err("account-invalid-type")
    }
}

/// Trims a submitted `last_four`; blank comes back as `""` (no value / clear it). Otherwise 1-4
/// ASCII letters or digits — mirrors the `accounts_last_four_valid` CHECK constraint.
fn validate_last_four(last_four: &str) -> Result<String, &'static str> {
    let trimmed = last_four.trim();
    if trimmed.chars().count() <= 4 && trimmed.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(trimmed.to_string())
    } else {
        Err("account-invalid-last-four")
    }
}

/// Uppercases a submitted ISO 4217 code ("cad" → "CAD"); must be exactly 3 ASCII letters.
fn validate_currency(currency: &str) -> Result<String, &'static str> {
    let upper = currency.trim().to_ascii_uppercase();
    if upper.len() == 3 && upper.chars().all(|c| c.is_ascii_uppercase()) {
        Ok(upper)
    } else {
        Err("account-invalid-currency")
    }
}

/// Applies `validate` to an optional field, keeping `None` as `None`.
fn validate_optional(
    value: Option<&str>,
    validate: fn(&str) -> Result<String, &'static str>,
) -> Result<Option<String>, &'static str> {
    value.map(validate).transpose()
}

/// `POST /accounts` — create an account in the caller's household.
async fn create_account(
    new_account: web::Json<NewAccount>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let validated = (|| {
        Ok::<_, &'static str>(NewAccount {
            institution_id: new_account.institution_id,
            name: validate_name(&new_account.name)?,
            r#type: validate_type(&new_account.r#type)?,
            last_four: validate_optional(new_account.last_four.as_deref(), validate_last_four)?
                .filter(|last_four| !last_four.is_empty()),
            currency: validate_optional(new_account.currency.as_deref(), validate_currency)?,
        })
    })();
    let new_account = match validated {
        Ok(new_account) => new_account,
        Err(message_id) => {
            return error_response(&l10n, &locale, StatusCode::BAD_REQUEST, message_id)
        }
    };

    match repository::create(&pool, household_id, &new_account).await {
        Ok(account) => HttpResponse::Created()
            .insert_header(("Location", format!("/accounts/{}", account.id)))
            .json(account),
        Err(sqlx::Error::RowNotFound) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "institution-not-found",
            new_account.institution_id as u32,
        ),
        Err(e) => {
            error!("failed to create account error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /accounts` — list the caller's household's accounts, grouped by institution.
async fn list_accounts(
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::list(&pool, household_id).await {
        Ok(accounts) => HttpResponse::Ok().json(accounts),
        Err(e) => {
            error!("failed to list accounts error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /accounts/{id}` — fetch a single account.
async fn get_account(
    path: web::Path<AccountIdPath>,
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

    match repository::get(&pool, household_id, id as i32).await {
        Ok(Some(account)) => HttpResponse::Ok().json(account),
        Ok(None) => not_found_response(&l10n, &locale, "account-not-found", id),
        Err(e) => {
            error!("failed to get account id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /accounts/{id}` — partially update an account; unset fields are left unchanged.
async fn update_account(
    path: web::Path<AccountIdPath>,
    patch: web::Json<AccountPatch>,
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

    let validated = (|| {
        Ok::<_, &'static str>(AccountPatch {
            institution_id: patch.institution_id,
            name: validate_optional(patch.name.as_deref(), validate_name)?,
            r#type: validate_optional(patch.r#type.as_deref(), validate_type)?,
            last_four: validate_optional(patch.last_four.as_deref(), validate_last_four)?,
            currency: validate_optional(patch.currency.as_deref(), validate_currency)?,
        })
    })();
    let patch = match validated {
        Ok(patch) => patch,
        Err(message_id) => {
            return error_response(&l10n, &locale, StatusCode::BAD_REQUEST, message_id)
        }
    };

    match repository::update(&pool, household_id, id as i32, &patch).await {
        Ok(Some(account)) => HttpResponse::Ok().json(account),
        Ok(None) => not_found_response(&l10n, &locale, "account-not-found", id),
        Err(sqlx::Error::RowNotFound) => error_response_with_n(
            &l10n,
            &locale,
            StatusCode::BAD_REQUEST,
            "institution-not-found",
            patch.institution_id.unwrap_or_default() as u32,
        ),
        Err(e) => {
            error!("failed to update account id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /accounts/{id}` — delete an account. Fails while any transaction is still in it.
async fn delete_account(
    path: web::Path<AccountIdPath>,
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

    match repository::delete(&pool, household_id, id as i32).await {
        Ok(true) => HttpResponse::NoContent().finish(),
        Ok(false) => not_found_response(&l10n, &locale, "account-not-found", id),
        Err(e) if is_foreign_key_violation(&e) => {
            error_response_with_n(&l10n, &locale, StatusCode::CONFLICT, "account-in-use", id)
        }
        Err(e) => {
            error!("failed to delete account id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the accounts feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/accounts", web::get().to(list_accounts))
        .route("/accounts", web::post().to(create_account))
        .route("/accounts/{id}", web::get().to(get_account))
        .route("/accounts/{id}", web::patch().to(update_account))
        .route("/accounts/{id}", web::delete().to(delete_account));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_last_four_accepts_up_to_four_alphanumerics() {
        assert_eq!(validate_last_four(" 40R3 "), Ok("40R3".to_string()));
        assert_eq!(validate_last_four(""), Ok(String::new()));
        assert!(validate_last_four("12345").is_err());
        assert!(validate_last_four("12-3").is_err());
    }

    #[test]
    fn validate_currency_uppercases_a_three_letter_code() {
        assert_eq!(validate_currency("usd"), Ok("USD".to_string()));
        assert!(validate_currency("US").is_err());
        assert!(validate_currency("U$D").is_err());
    }
}
