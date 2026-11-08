//! Matching and applying rules — pure functions, shared by the preview, applying a rule to
//! existing transactions, and statement import.

use std::collections::HashSet;

use super::model::{
    AmountCondition, Rule, RuleActions, RuleConditions, RuleTarget, TextCondition, TextOperator,
};
use crate::features::accounts::Owner;
use crate::features::transactions::model::{AmountOp, Transaction};

impl RuleTarget {
    /// A stored transaction as rules see it.
    pub fn from_transaction(transaction: &Transaction) -> Self {
        let mut tag_ids: Vec<i32> = transaction.tags.iter().map(|tag| tag.id).collect();
        tag_ids.sort_unstable();
        tag_ids.dedup();
        RuleTarget {
            original_statement: transaction.original_statement.clone(),
            amount: transaction.amount,
            account_id: Some(transaction.account_id),
            merchant_id: Some(transaction.merchant_id),
            category_id: Some(transaction.category_id),
            owner_user_id: transaction.owner_user_id,
            tag_ids,
            reviewed: transaction.reviewed,
            review_assignee_user_id: transaction.review_assignee_user_id,
        }
    }
}

/// Whether `conditions` has any condition set — a rule without one never matches.
pub fn matches_anything(conditions: &RuleConditions) -> bool {
    conditions.original_statement.is_some()
        || conditions.merchant_ids.is_some()
        || conditions.amount.is_some()
        || conditions.category_ids.is_some()
        || conditions.owners.is_some()
        || conditions.account_ids.is_some()
}

/// Whether `target` matches every condition set in `conditions`. With none set, never.
pub fn matches(conditions: &RuleConditions, target: &RuleTarget) -> bool {
    if !matches_anything(conditions) {
        return false;
    }
    let RuleConditions {
        original_statement,
        merchant_ids,
        amount,
        category_ids,
        owners,
        account_ids,
    } = conditions;

    let in_list = |ids: &Option<Vec<i32>>, id: Option<i32>| match ids {
        None => true,
        Some(ids) => id.is_some_and(|id| ids.contains(&id)),
    };

    original_statement.as_ref().is_none_or(|texts| {
        texts
            .iter()
            .any(|text| text_matches(text, &target.original_statement))
    }) && in_list(merchant_ids, target.merchant_id)
        && amount
            .as_ref()
            .is_none_or(|amount| amount_matches(amount, target))
        && in_list(category_ids, target.category_id)
        && owners
            .as_ref()
            .is_none_or(|owners| owners.contains(&Owner::from_user_id(target.owner_user_id)))
        && in_list(account_ids, target.account_id)
}

fn text_matches(condition: &TextCondition, text: &str) -> bool {
    let value = condition.value.trim().to_lowercase();
    let text = text.trim().to_lowercase();
    match condition.operator {
        TextOperator::Contains => text.contains(&value),
        TextOperator::Equals => text == value,
    }
}

fn amount_matches(condition: &AmountCondition, target: &RuleTarget) -> bool {
    let amount = target.amount;
    match condition.operator {
        AmountOp::GreaterThan => amount > condition.value,
        AmountOp::LessThan => amount < condition.value,
        AmountOp::EqualTo => amount == condition.value,
        AmountOp::Between => {
            amount >= condition.value && condition.value_max.is_none_or(|max| amount <= max)
        }
    }
}

/// Applies every action set in `actions` to `target`.
pub fn apply(actions: &RuleActions, target: &mut RuleTarget) {
    if let Some(merchant_id) = actions.merchant_id {
        target.merchant_id = Some(merchant_id);
    }
    if let Some(category_id) = actions.category_id {
        target.category_id = Some(category_id);
    }
    if let Some(owner) = actions.owner {
        target.owner_user_id = owner.user_id();
    }
    if let Some(tag_ids) = &actions.tag_ids {
        target.tag_ids.extend(tag_ids);
        target.tag_ids.sort_unstable();
        target.tag_ids.dedup();
    }
    if let Some(review) = actions.review {
        target.reviewed = review.reviewed;
        if !review.reviewed {
            target.review_assignee_user_id =
                review.assignee.and_then(|assignee| assignee.user_id());
        }
    }
}

/// Runs `rules` (already in their order) on `target`, top to bottom: each rule that matches the
/// transaction as the rules above left it applies its actions, so a lower rule overwrites what a
/// higher one set. Returns the rules that matched.
pub fn run<'a>(rules: &'a [Rule], target: &mut RuleTarget) -> Vec<&'a Rule> {
    let mut applied = Vec::new();
    for rule in rules {
        if matches(&rule.conditions, target) {
            apply(&rule.actions, target);
            applied.push(rule);
        }
    }
    applied
}

/// The ids a household can use in a rule: to check a rule being saved (see `references_known`),
/// and to drop the actions of stored rules that point to something deleted since (see
/// `prune_actions`).
#[derive(Debug, Default)]
pub struct KnownIds {
    pub merchant_ids: HashSet<i32>,
    pub category_ids: HashSet<i32>,
    pub account_ids: HashSet<i32>,
    pub tag_ids: HashSet<i32>,
    pub member_ids: HashSet<i32>,
}

/// Drops each action of `rules` that points to a merchant, category, tag or member no longer in
/// `known`, so a stored rule never sets something that's gone. Conditions need no pruning: a
/// deleted id just never matches.
pub fn prune_actions(rules: &mut [Rule], known: &KnownIds) {
    let known_member = |id: Option<i32>| id.is_none_or(|id| known.member_ids.contains(&id));
    for rule in rules {
        let actions = &mut rule.actions.0;
        actions.merchant_id = actions
            .merchant_id
            .filter(|id| known.merchant_ids.contains(id));
        actions.category_id = actions
            .category_id
            .filter(|id| known.category_ids.contains(id));
        actions.owner = actions.owner.filter(|owner| known_member(owner.user_id()));
        if let Some(tag_ids) = &mut actions.tag_ids {
            tag_ids.retain(|id| known.tag_ids.contains(id));
        }
        actions.review = actions
            .review
            .filter(|review| known_member(review.assignee.and_then(|assignee| assignee.user_id())));
    }
}

/// Whether every id `conditions` and `actions` name is one of the household's.
pub fn references_known(
    conditions: &RuleConditions,
    actions: &RuleActions,
    known: &KnownIds,
) -> bool {
    let all_in = |ids: &Option<Vec<i32>>, set: &HashSet<i32>| {
        ids.iter().flatten().all(|id| set.contains(id))
    };
    let member_known = |id: Option<i32>| id.is_none_or(|id| known.member_ids.contains(&id));
    all_in(&conditions.merchant_ids, &known.merchant_ids)
        && all_in(&conditions.category_ids, &known.category_ids)
        && all_in(&conditions.account_ids, &known.account_ids)
        && conditions
            .owners
            .iter()
            .flatten()
            .all(|owner| member_known(owner.user_id()))
        && actions
            .merchant_id
            .is_none_or(|id| known.merchant_ids.contains(&id))
        && actions
            .category_id
            .is_none_or(|id| known.category_ids.contains(&id))
        && actions
            .owner
            .is_none_or(|owner| member_known(owner.user_id()))
        && all_in(&actions.tag_ids, &known.tag_ids)
        && actions.review.is_none_or(|review| {
            member_known(review.assignee.and_then(|assignee| assignee.user_id()))
        })
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use rust_decimal::Decimal;
    use sqlx::types::Json;

    use super::*;
    use crate::features::rules::model::{ReviewAction, TextCondition};
    use crate::features::transactions::model::ReviewAssignee;

    fn target() -> RuleTarget {
        RuleTarget {
            original_statement: "SPOTIFY P1234 STOCKHOLM".to_string(),
            amount: Decimal::new(1299, 2),
            account_id: Some(5),
            merchant_id: Some(42),
            category_id: Some(3),
            owner_user_id: Some(7),
            tag_ids: vec![1],
            reviewed: true,
            review_assignee_user_id: None,
        }
    }

    fn rule(conditions: RuleConditions, actions: RuleActions) -> Rule {
        Rule {
            id: 1,
            household_id: 1,
            conditions: Json(conditions),
            actions: Json(actions),
            sort_order: 0,
            last_applied_at: None,
            last_applied_count: 0,
            created_at: Utc::now(),
        }
    }

    fn statement(operator: TextOperator, value: &str) -> TextCondition {
        TextCondition {
            operator,
            value: value.to_string(),
        }
    }

    #[test]
    fn no_condition_never_matches() {
        assert!(!matches(&RuleConditions::default(), &target()));
    }

    #[test]
    fn text_matches_ignoring_case_and_any_value() {
        let conditions = RuleConditions {
            original_statement: Some(vec![
                statement(TextOperator::Contains, "netflix"),
                statement(TextOperator::Contains, "spotify"),
            ]),
            ..Default::default()
        };
        assert!(matches(&conditions, &target()));

        let exact = RuleConditions {
            original_statement: Some(vec![statement(TextOperator::Equals, "spotify")]),
            ..Default::default()
        };
        assert!(!matches(&exact, &target()));
    }

    #[test]
    fn every_condition_must_match() {
        let conditions = RuleConditions {
            original_statement: Some(vec![statement(TextOperator::Contains, "spotify")]),
            category_ids: Some(vec![99]),
            ..Default::default()
        };
        assert!(!matches(&conditions, &target()));
    }

    #[test]
    fn amount_and_owner_conditions() {
        let between = RuleConditions {
            amount: Some(AmountCondition {
                operator: AmountOp::Between,
                value: Decimal::new(10, 0),
                value_max: Some(Decimal::new(13, 0)),
            }),
            owners: Some(vec![Owner::Shared, Owner::Member(7)]),
            ..Default::default()
        };
        assert!(matches(&between, &target()));

        let shared_only = RuleConditions {
            owners: Some(vec![Owner::Shared]),
            ..Default::default()
        };
        assert!(!matches(&shared_only, &target()));
    }

    #[test]
    fn a_line_without_a_merchant_never_matches_merchant_ids() {
        let conditions = RuleConditions {
            merchant_ids: Some(vec![42]),
            ..Default::default()
        };
        let mut line = target();
        line.merchant_id = None;
        assert!(!matches(&conditions, &line));
    }

    #[test]
    fn apply_adds_tags_and_sends_for_review() {
        let mut target = target();
        apply(
            &RuleActions {
                tag_ids: Some(vec![4, 1]),
                review: Some(ReviewAction {
                    reviewed: false,
                    assignee: Some(ReviewAssignee::Member(7)),
                }),
                owner: Some(Owner::Shared),
                ..Default::default()
            },
            &mut target,
        );
        assert_eq!(target.tag_ids, vec![1, 4]);
        assert!(!target.reviewed);
        assert_eq!(target.review_assignee_user_id, Some(7));
        assert_eq!(target.owner_user_id, None);
    }

    #[test]
    fn lower_rules_see_earlier_changes_and_win() {
        let rules = [
            rule(
                RuleConditions {
                    merchant_ids: Some(vec![42]),
                    ..Default::default()
                },
                RuleActions {
                    category_id: Some(10),
                    ..Default::default()
                },
            ),
            rule(
                RuleConditions {
                    category_ids: Some(vec![10]),
                    ..Default::default()
                },
                RuleActions {
                    category_id: Some(11),
                    tag_ids: Some(vec![2]),
                    ..Default::default()
                },
            ),
        ];
        let mut target = target();
        assert_eq!(run(&rules, &mut target).len(), 2);
        assert_eq!(target.category_id, Some(11));
        assert_eq!(target.tag_ids, vec![1, 2]);
    }

    #[test]
    fn prune_drops_actions_pointing_to_deleted_ids() {
        let mut rules = [rule(
            RuleConditions::default(),
            RuleActions {
                merchant_id: Some(42),
                category_id: Some(3),
                owner: Some(Owner::Member(8)),
                tag_ids: Some(vec![1, 2]),
                review: None,
            },
        )];
        let known = KnownIds {
            merchant_ids: [42].into(),
            category_ids: HashSet::new(),
            account_ids: HashSet::new(),
            tag_ids: [2].into(),
            member_ids: [7].into(),
        };
        prune_actions(&mut rules, &known);
        let actions = &rules[0].actions.0;
        assert_eq!(actions.merchant_id, Some(42));
        assert_eq!(actions.category_id, None);
        assert_eq!(actions.owner, None);
        assert_eq!(actions.tag_ids, Some(vec![2]));
    }
}
