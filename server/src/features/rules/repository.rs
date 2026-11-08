use std::collections::HashMap;

use sqlx::types::Json;
use sqlx::PgPool;

use super::engine::{prune_actions, KnownIds};
use super::model::{Rule, RuleActions, RuleConditions};

const SELECT_COLUMNS: &str =
    "id, household_id, conditions, actions, sort_order, last_applied_at, last_applied_count, created_at";

/// Inserts a new rule, scoped to `household_id`, at the bottom of the household's order, and
/// returns the created row.
pub async fn create(
    pool: &PgPool,
    household_id: i32,
    conditions: &RuleConditions,
    actions: &RuleActions,
) -> Result<Rule, sqlx::Error> {
    sqlx::query_as::<_, Rule>(&format!(
        "INSERT INTO rules (household_id, conditions, actions, sort_order)
         VALUES (
             $1, $2, $3,
             (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM rules WHERE household_id = $1)
         )
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(household_id)
    .bind(Json(conditions))
    .bind(Json(actions))
    .fetch_one(pool)
    .await
}

/// Lists a household's rules in the order they run (see `reorder`).
pub async fn list(pool: &PgPool, household_id: i32) -> Result<Vec<Rule>, sqlx::Error> {
    sqlx::query_as::<_, Rule>(&format!(
        "SELECT {SELECT_COLUMNS} FROM rules WHERE household_id = $1 ORDER BY sort_order, id"
    ))
    .bind(household_id)
    .fetch_all(pool)
    .await
}

/// Fetches a single rule by id, scoped to `household_id`, or `None` if it doesn't exist (including
/// when it belongs to a different household).
pub async fn get(pool: &PgPool, household_id: i32, id: i32) -> Result<Option<Rule>, sqlx::Error> {
    sqlx::query_as::<_, Rule>(&format!(
        "SELECT {SELECT_COLUMNS} FROM rules WHERE id = $1 AND household_id = $2"
    ))
    .bind(id)
    .bind(household_id)
    .fetch_optional(pool)
    .await
}

/// Replaces a rule's conditions and/or actions (only `Some` ones change), scoped to
/// `household_id`, and returns the updated row, or `None` if the id doesn't exist (including when
/// it belongs to a different household).
pub async fn update(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    conditions: Option<&RuleConditions>,
    actions: Option<&RuleActions>,
) -> Result<Option<Rule>, sqlx::Error> {
    sqlx::query_as::<_, Rule>(&format!(
        "UPDATE rules
         SET conditions = COALESCE($3, conditions),
             actions = COALESCE($4, actions)
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(conditions.map(Json))
    .bind(actions.map(Json))
    .fetch_optional(pool)
    .await
}

/// Records that a rule was just applied to existing transactions and changed `count` of them, and
/// returns the updated row.
pub async fn mark_applied(
    pool: &PgPool,
    household_id: i32,
    id: i32,
    count: usize,
) -> Result<Option<Rule>, sqlx::Error> {
    sqlx::query_as::<_, Rule>(&format!(
        "UPDATE rules SET last_applied_at = now(), last_applied_count = $3
         WHERE id = $1 AND household_id = $2
         RETURNING {SELECT_COLUMNS}"
    ))
    .bind(id)
    .bind(household_id)
    .bind(count as i32)
    .fetch_optional(pool)
    .await
}

/// Sets each id in `rule_ids`' `sort_order` to its index in that list, scoped to `household_id` —
/// ids that don't exist or belong to a different household are silently skipped. A rule left out
/// keeps its existing `sort_order`.
pub async fn reorder(
    pool: &PgPool,
    household_id: i32,
    rule_ids: &[i32],
) -> Result<(), sqlx::Error> {
    if rule_ids.is_empty() {
        return Ok(());
    }

    let positions: Vec<i32> = (0..rule_ids.len() as i32).collect();
    sqlx::query(
        "UPDATE rules
         SET sort_order = data.position
         FROM UNNEST($1::integer[], $2::integer[]) AS data(id, position)
         WHERE rules.id = data.id AND rules.household_id = $3",
    )
    .bind(rule_ids)
    .bind(&positions)
    .bind(household_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Deletes a rule by id, scoped to `household_id`. Returns `true` if a row was deleted, `false` if
/// the id didn't exist (including when it belongs to a different household).
pub async fn delete(pool: &PgPool, household_id: i32, id: i32) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM rules WHERE id = $1 AND household_id = $2")
        .bind(id)
        .bind(household_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Every merchant (common ones included), category, account, tag and member id the household can
/// use in a rule.
pub async fn known_ids(pool: &PgPool, household_id: i32) -> Result<KnownIds, sqlx::Error> {
    let ids = |sql: &'static str| {
        sqlx::query_scalar::<_, i32>(sql)
            .bind(household_id)
            .fetch_all(pool)
    };
    Ok(KnownIds {
        merchant_ids: ids(
            "SELECT id FROM merchants WHERE household_id IS NULL OR household_id = $1",
        )
        .await?
        .into_iter()
        .collect(),
        category_ids: ids("SELECT id FROM categories WHERE household_id = $1")
            .await?
            .into_iter()
            .collect(),
        account_ids: ids("SELECT id FROM accounts WHERE household_id = $1")
            .await?
            .into_iter()
            .collect(),
        tag_ids: ids("SELECT id FROM tags WHERE household_id = $1")
            .await?
            .into_iter()
            .collect(),
        member_ids: ids("SELECT id FROM users WHERE household_id = $1")
            .await?
            .into_iter()
            .collect(),
    })
}

/// The household's rules in the order they run, without the actions that point to something
/// deleted since they were saved — ready for `engine::run`.
pub async fn list_runnable(pool: &PgPool, household_id: i32) -> Result<Vec<Rule>, sqlx::Error> {
    let mut rules = list(pool, household_id).await?;
    if !rules.is_empty() {
        prune_actions(&mut rules, &known_ids(pool, household_id).await?);
    }
    Ok(rules)
}

/// Each of the household's accounts' owner (`None` when Shared), for an owner condition on a
/// transaction that isn't stored yet.
pub async fn account_owners(
    pool: &PgPool,
    household_id: i32,
) -> Result<HashMap<i32, Option<i32>>, sqlx::Error> {
    let rows: Vec<(i32, Option<i32>)> =
        sqlx::query_as("SELECT id, owner_user_id FROM accounts WHERE household_id = $1")
            .bind(household_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}
