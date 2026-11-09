//! HTTP API for rules: JSON CRUD backed by Postgres, scoped to the caller's household, plus a
//! preview and applying a rule to existing transactions.

use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, Responder};
use log::error;
use serde::Deserialize;
use sqlx::PgPool;

use super::engine::{self, references_known};
use super::model::{
    Preview, PreviewRequest, PreviewTransaction, ReorderRulesRequest, Rule, RuleActions,
    RuleConditions, RuleInput, RuleTarget,
};
use super::repository;
use crate::features::auth::CurrentUser;
use crate::features::transactions::model::{
    AmountOp, BulkTransactionPatch, ReviewAssignee, Transaction, TransactionFilter,
};
use crate::features::transactions::repository as transactions_repository;
use crate::shared::http_error::{error_response, internal_error_response, not_found_response};
use crate::shared::l10n::L10n;

/// Most matching transactions `POST /rules/preview` sends back; `match_count` still counts them all.
const PREVIEW_LIMIT: usize = 500;

/// Rule id path (`/rules/{id}`)
#[derive(Deserialize)]
struct RuleIdPath {
    id: u32,
}

/// Why a rule can't be saved, as the Fluent message id to answer with.
fn invalid_reason(conditions: &RuleConditions, actions: &RuleActions) -> Option<&'static str> {
    let filled = |ids: &Option<Vec<i32>>| ids.as_ref().is_some_and(|ids| !ids.is_empty());
    let texts_ok = conditions.original_statement.as_ref().is_none_or(|texts| {
        !texts.is_empty() && texts.iter().all(|text| !text.value.trim().is_empty())
    });
    let amount_ok = conditions.amount.as_ref().is_none_or(|amount| {
        amount.operator != AmountOp::Between
            || amount.value_max.is_some_and(|max| max >= amount.value)
    });
    let lists_ok = [
        &conditions.merchant_ids,
        &conditions.category_ids,
        &conditions.account_ids,
    ]
    .into_iter()
    .all(|ids| ids.is_none() || filled(ids))
        && conditions
            .owners
            .as_ref()
            .is_none_or(|owners| !owners.is_empty());
    if !texts_ok || !amount_ok || !lists_ok {
        return Some("rule-invalid-condition");
    }
    if !engine::matches_anything(conditions) {
        return Some("rule-no-condition");
    }
    let has_action = actions.merchant_id.is_some()
        || actions.category_id.is_some()
        || actions.owner.is_some()
        || filled(&actions.tag_ids)
        || actions.review.is_some();
    if !has_action {
        return Some("rule-no-action");
    }
    None
}

/// Checks `conditions`/`actions` can be saved for `household_id`, answering the `400` (or `500`)
/// to send back when not.
async fn check(
    pool: &PgPool,
    household_id: i32,
    conditions: &RuleConditions,
    actions: &RuleActions,
    l10n: &L10n,
    locale: &unic_langid::LanguageIdentifier,
) -> Result<(), HttpResponse> {
    if let Some(reason) = invalid_reason(conditions, actions) {
        return Err(error_response(
            l10n,
            locale,
            StatusCode::BAD_REQUEST,
            reason,
        ));
    }
    match repository::known_ids(pool, household_id).await {
        Ok(known) if references_known(conditions, actions, &known) => Ok(()),
        Ok(_) => Err(error_response(
            l10n,
            locale,
            StatusCode::BAD_REQUEST,
            "rule-unknown-reference",
        )),
        Err(e) => {
            error!("failed to load ids to check a rule error={e}");
            Err(internal_error_response(l10n, locale))
        }
    }
}

/// The household's transactions (most recent first) that match `conditions`, each with what
/// `actions` would make it.
async fn matching(
    pool: &PgPool,
    household_id: i32,
    user_id: i32,
    conditions: &RuleConditions,
    actions: &RuleActions,
) -> Result<Vec<(Transaction, RuleTarget, bool)>, sqlx::Error> {
    let transactions =
        transactions_repository::list(pool, household_id, user_id, &TransactionFilter::default())
            .await?;
    Ok(transactions
        .into_iter()
        .filter_map(|transaction| {
            let before = RuleTarget::from_transaction(&transaction);
            if !engine::matches(conditions, &before) {
                return None;
            }
            let mut after = before.clone();
            engine::apply(actions, &mut after);
            let changed = after != before;
            Some((transaction, after, changed))
        })
        .collect())
}

/// Applies `rule` alone to the household's existing transactions that match it, on behalf of
/// `user_id`, and returns the rule with how many it changed.
async fn apply_to_existing(
    pool: &PgPool,
    household_id: i32,
    user_id: i32,
    rule: &Rule,
) -> Result<Rule, sqlx::Error> {
    let ids: Vec<i64> = matching(pool, household_id, user_id, &rule.conditions, &rule.actions)
        .await?
        .into_iter()
        .filter(|(_, _, changed)| *changed)
        .map(|(transaction, _, _)| transaction.id)
        .collect();

    if !ids.is_empty() {
        let actions = &rule.actions;
        let review = actions.review;
        let patch = BulkTransactionPatch {
            date: None,
            merchant_id: actions.merchant_id,
            category_id: actions.category_id,
            account_id: None,
            owner: actions.owner,
            reviewed: review.map(|review| review.reviewed),
            review_assignee: review
                .filter(|review| !review.reviewed)
                .map(|review| review.assignee.unwrap_or(ReviewAssignee::Anyone)),
            tag_ids: actions.tag_ids.clone(),
        };
        transactions_repository::bulk_update(pool, household_id, user_id, &ids, &patch).await?;
    }

    Ok(
        repository::mark_applied(pool, household_id, rule.id, ids.len())
            .await?
            .unwrap_or_else(|| rule.clone()),
    )
}

/// `POST /rules` — create a rule at the bottom of the caller's household's rules, and apply it to
/// existing transactions when `apply_to_existing` is set.
async fn create_rule(
    body: web::Json<RuleInput>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };
    let Some(conditions) = &body.conditions else {
        return error_response(&l10n, &locale, StatusCode::BAD_REQUEST, "rule-no-condition");
    };
    let Some(actions) = &body.actions else {
        return error_response(&l10n, &locale, StatusCode::BAD_REQUEST, "rule-no-action");
    };
    if let Err(response) = check(&pool, household_id, conditions, actions, &l10n, &locale).await {
        return response;
    }

    let rule = match repository::create(&pool, household_id, conditions, actions).await {
        Ok(rule) => rule,
        Err(e) => {
            error!("failed to create rule error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };
    let rule = if body.apply_to_existing {
        match apply_to_existing(&pool, household_id, current_user.user.id, &rule).await {
            Ok(rule) => rule,
            Err(e) => {
                error!("failed to apply new rule id={} error={e}", rule.id);
                return internal_error_response(&l10n, &locale);
            }
        }
    } else {
        rule
    };

    HttpResponse::Created()
        .insert_header(("Location", format!("/rules/{}", rule.id)))
        .json(rule)
}

/// `GET /rules` — list the caller's household's rules, in the order they run.
async fn list_rules(
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
        Ok(rules) => HttpResponse::Ok().json(rules),
        Err(e) => {
            error!("failed to list rules error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /rules/reorder` — set the order the caller's household's rules run in to `rule_ids`.
/// Registered ahead of `/rules/{id}` so "reorder" is never parsed as an id.
async fn reorder_rules(
    body: web::Json<ReorderRulesRequest>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match repository::reorder(&pool, household_id, &body.rule_ids).await {
        Ok(()) => match repository::list(&pool, household_id).await {
            Ok(rules) => HttpResponse::Ok().json(rules),
            Err(e) => {
                error!("failed to list rules after reorder error={e}");
                internal_error_response(&l10n, &locale)
            }
        },
        Err(e) => {
            error!("failed to reorder rules error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `POST /rules/preview` — the caller's household's transactions a rule (saved or not) matches,
/// with what it would change. Stores nothing.
async fn preview_rule(
    body: web::Json<PreviewRequest>,
    current_user: CurrentUser,
    pool: web::Data<PgPool>,
    l10n: web::Data<L10n>,
) -> impl Responder {
    let locale = l10n.locale();
    let household_id = match current_user.require_household_id(&l10n, &locale) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match matching(
        &pool,
        household_id,
        current_user.user.id,
        &body.conditions,
        &body.actions,
    )
    .await
    {
        Ok(matches) => {
            let match_count = matches.len();
            let change_count = matches.iter().filter(|(_, _, changed)| *changed).count();
            let transactions = matches
                .into_iter()
                .take(PREVIEW_LIMIT)
                .map(|(before, after, _)| PreviewTransaction { before, after })
                .collect();
            HttpResponse::Ok().json(Preview {
                match_count,
                change_count,
                transactions,
            })
        }
        Err(e) => {
            error!("failed to preview rule error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `GET /rules/{id}` — fetch a single rule from the caller's household.
async fn get_rule(
    path: web::Path<RuleIdPath>,
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
        Ok(Some(rule)) => HttpResponse::Ok().json(rule),
        Ok(None) => not_found_response(&l10n, &locale, "rule-not-found", id),
        Err(e) => {
            error!("failed to get rule id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `PATCH /rules/{id}` — replace a rule's conditions and/or actions, and apply it to existing
/// transactions when `apply_to_existing` is set.
async fn update_rule(
    path: web::Path<RuleIdPath>,
    body: web::Json<RuleInput>,
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

    let existing = match repository::get(&pool, household_id, id as i32).await {
        Ok(Some(rule)) => rule,
        Ok(None) => return not_found_response(&l10n, &locale, "rule-not-found", id),
        Err(e) => {
            error!("failed to get rule id={id} error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };
    let conditions = body.conditions.as_ref().unwrap_or(&existing.conditions);
    let actions = body.actions.as_ref().unwrap_or(&existing.actions);
    if let Err(response) = check(&pool, household_id, conditions, actions, &l10n, &locale).await {
        return response;
    }

    let rule = match repository::update(
        &pool,
        household_id,
        id as i32,
        body.conditions.as_ref(),
        body.actions.as_ref(),
    )
    .await
    {
        Ok(Some(rule)) => rule,
        Ok(None) => return not_found_response(&l10n, &locale, "rule-not-found", id),
        Err(e) => {
            error!("failed to update rule id={id} error={e}");
            return internal_error_response(&l10n, &locale);
        }
    };
    if !body.apply_to_existing {
        return HttpResponse::Ok().json(rule);
    }
    match apply_to_existing(&pool, household_id, current_user.user.id, &rule).await {
        Ok(rule) => HttpResponse::Ok().json(rule),
        Err(e) => {
            error!("failed to apply rule id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// `DELETE /rules/{id}` — delete a rule from the caller's household. Transactions it already
/// changed stay as they are.
async fn delete_rule(
    path: web::Path<RuleIdPath>,
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
        Ok(false) => not_found_response(&l10n, &locale, "rule-not-found", id),
        Err(e) => {
            error!("failed to delete rule id={id} error={e}");
            internal_error_response(&l10n, &locale)
        }
    }
}

/// Registers the rules feature's routes.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/rules", web::get().to(list_rules))
        .route("/rules", web::post().to(create_rule))
        .route("/rules/reorder", web::patch().to(reorder_rules))
        .route("/rules/preview", web::post().to(preview_rule))
        .route("/rules/{id}", web::get().to(get_rule))
        .route("/rules/{id}", web::patch().to(update_rule))
        .route("/rules/{id}", web::delete().to(delete_rule));
}
