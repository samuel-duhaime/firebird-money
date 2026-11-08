use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::types::Json;
use sqlx::FromRow;

use crate::features::accounts::Owner;
use crate::features::transactions::model::{AmountOp, ReviewAssignee, Transaction};

/// A single row in the `rules` table: "if a transaction matches every condition, apply every
/// action". A household's rules run top to bottom (lowest `sort_order` first), each one seeing the
/// transaction as the rules above it left it, so a lower rule can overwrite what a higher one set.
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Rule {
    pub id: i32,
    pub household_id: i32,
    pub conditions: Json<RuleConditions>,
    pub actions: Json<RuleActions>,
    /// This household's chosen order for its rules (see `PATCH /rules/reorder`) — lower runs first.
    pub sort_order: i32,
    /// When the rule was last applied to existing transactions (`apply_to_existing`), and how many
    /// it changed then.
    pub last_applied_at: Option<DateTime<Utc>>,
    pub last_applied_count: i32,
    pub created_at: DateTime<Utc>,
}

/// What a transaction must match. Every field that's set must match (`None` means "don't check");
/// within a list, matching any one value is enough. A rule with no condition set never matches.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleConditions {
    /// Against the raw statement text, ignoring case.
    pub original_statement: Option<Vec<TextCondition>>,
    pub merchant_ids: Option<Vec<i32>>,
    pub amount: Option<AmountCondition>,
    pub category_ids: Option<Vec<i32>>,
    pub owners: Option<Vec<Owner>>,
    pub account_ids: Option<Vec<i32>>,
}

/// One accepted text in `RuleConditions::original_statement`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextCondition {
    pub operator: TextOperator,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextOperator {
    Contains,
    /// The whole text, shown as "exactly matches".
    Equals,
}

/// `RuleConditions::amount`. Amounts are always positive (direction comes from the category), so
/// there's no debit/credit here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AmountCondition {
    pub operator: AmountOp,
    pub value: Decimal,
    /// Inclusive upper bound, only read (and required) when `operator` is `Between`.
    pub value_max: Option<Decimal>,
}

/// What a matching transaction gets. `None` leaves that field as is.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RuleActions {
    /// Renames the merchant: moves the transaction to this existing merchant.
    pub merchant_id: Option<i32>,
    pub category_id: Option<i32>,
    pub owner: Option<Owner>,
    /// *Added* to the transaction's tags; never removes one.
    pub tag_ids: Option<Vec<i32>>,
    pub review: Option<ReviewAction>,
}

/// `RuleActions::review`: mark it reviewed, or send it for review to `assignee` (Anyone when
/// absent). `assignee` is ignored when `reviewed` is true.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewAction {
    pub reviewed: bool,
    pub assignee: Option<ReviewAssignee>,
}

/// Body for `POST /rules` and `PATCH /rules/{id}`. A new rule always starts at the bottom of the
/// household's order. On `PATCH`, an absent `conditions`/`actions` is left unchanged; a given one
/// replaces the whole set.
#[derive(Debug, Deserialize)]
pub struct RuleInput {
    pub conditions: Option<RuleConditions>,
    pub actions: Option<RuleActions>,
    /// Also apply this rule (alone) to the household's existing transactions that match it, once
    /// saved.
    #[serde(default)]
    pub apply_to_existing: bool,
}

/// Body for `PATCH /rules/reorder` — the household's rules, in their new order. Ids outside the
/// caller's household are silently skipped, and a rule left out keeps its `sort_order`, same as
/// `PATCH /tags/reorder`.
#[derive(Debug, Deserialize)]
pub struct ReorderRulesRequest {
    pub rule_ids: Vec<i32>,
}

/// Body for `POST /rules/preview`: a rule that isn't saved (or not with these changes) yet.
#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub conditions: RuleConditions,
    pub actions: RuleActions,
}

/// Response for `POST /rules/preview`. Nothing is stored.
#[derive(Debug, Serialize)]
pub struct Preview {
    /// How many of the household's transactions match.
    pub match_count: usize,
    /// How many of those the rule would actually change.
    pub change_count: usize,
    /// The matching transactions, most recent first, at most `PREVIEW_LIMIT` of them.
    pub transactions: Vec<PreviewTransaction>,
}

/// One matching transaction in a `Preview`: as it is now, and what the rule would make it.
#[derive(Debug, Serialize)]
pub struct PreviewTransaction {
    pub before: Transaction,
    pub after: RuleTarget,
}

/// The parts of a transaction rules read and write — a stored transaction or a statement line
/// still being reviewed alike. `None` ids are a line's merchant or account that doesn't exist yet,
/// which no condition matches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuleTarget {
    #[serde(skip)]
    pub original_statement: String,
    #[serde(skip)]
    pub amount: Decimal,
    #[serde(skip)]
    pub account_id: Option<i32>,
    pub merchant_id: Option<i32>,
    pub category_id: Option<i32>,
    /// `None` when Shared.
    pub owner_user_id: Option<i32>,
    /// Sorted, without duplicates.
    pub tag_ids: Vec<i32>,
    pub reviewed: bool,
    /// Only meaningful while `reviewed` is false; `None` is Anyone.
    pub review_assignee_user_id: Option<i32>,
}
