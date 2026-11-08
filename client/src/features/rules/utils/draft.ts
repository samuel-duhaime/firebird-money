import type { Owner } from '../../accounts/utils/api';
import type { ReviewAssignee } from '../../transactions/utils/api';
import type {
  AmountOperator,
  Rule,
  RuleActions,
  RuleConditions,
  TextCondition,
} from './types';

/** What the review action sets: reviewed, or needs a review by Anyone or a member. */
export type ReviewChoice = 'reviewed' | ReviewAssignee;

/** The rule panel's form: each condition and action has its own switch, and keeps what was typed
 * even while switched off, so switching it back on brings it back. */
export interface RuleDraft {
  statementOn: boolean;
  statement: TextCondition[];
  merchantsOn: boolean;
  merchantIds: number[];
  amountOn: boolean;
  amountOperator: AmountOperator;
  amount: string;
  amountMax: string;
  categoriesOn: boolean;
  categoryIds: number[];
  ownersOn: boolean;
  owners: Owner[];
  accountsOn: boolean;
  accountIds: number[];

  renameOn: boolean;
  merchantId: number | null;
  categoryOn: boolean;
  categoryId: number | null;
  ownerOn: boolean;
  owner: Owner | null;
  tagsOn: boolean;
  tagIds: number[];
  reviewOn: boolean;
  review: ReviewChoice;
}

export const emptyDraft = (): RuleDraft => ({
  statementOn: false,
  statement: [{ operator: 'contains', value: '' }],
  merchantsOn: false,
  merchantIds: [],
  amountOn: false,
  amountOperator: 'greater_than',
  amount: '',
  amountMax: '',
  categoriesOn: false,
  categoryIds: [],
  ownersOn: false,
  owners: [],
  accountsOn: false,
  accountIds: [],
  renameOn: false,
  merchantId: null,
  categoryOn: false,
  categoryId: null,
  ownerOn: false,
  owner: null,
  tagsOn: false,
  tagIds: [],
  reviewOn: false,
  review: 'reviewed',
});

/** A draft with `conditions` and `actions` filled in, e.g. a saved rule being edited or one
 * suggested from a transaction edit. */
export const toDraft = (
  conditions: Partial<RuleConditions>,
  actions: Partial<RuleActions>,
): RuleDraft => {
  const draft = emptyDraft();
  if (conditions.original_statement) {
    draft.statementOn = true;
    draft.statement = conditions.original_statement;
  }
  if (conditions.merchant_ids) {
    draft.merchantsOn = true;
    draft.merchantIds = conditions.merchant_ids;
  }
  if (conditions.amount) {
    draft.amountOn = true;
    draft.amountOperator = conditions.amount.operator;
    draft.amount = conditions.amount.value;
    draft.amountMax = conditions.amount.value_max ?? '';
  }
  if (conditions.category_ids) {
    draft.categoriesOn = true;
    draft.categoryIds = conditions.category_ids;
  }
  if (conditions.owners) {
    draft.ownersOn = true;
    draft.owners = conditions.owners;
  }
  if (conditions.account_ids) {
    draft.accountsOn = true;
    draft.accountIds = conditions.account_ids;
  }
  if (actions.merchant_id != null) {
    draft.renameOn = true;
    draft.merchantId = actions.merchant_id;
  }
  if (actions.category_id != null) {
    draft.categoryOn = true;
    draft.categoryId = actions.category_id;
  }
  if (actions.owner != null) {
    draft.ownerOn = true;
    draft.owner = actions.owner;
  }
  if (actions.tag_ids) {
    draft.tagsOn = true;
    draft.tagIds = actions.tag_ids;
  }
  if (actions.review) {
    draft.reviewOn = true;
    draft.review = actions.review.reviewed
      ? 'reviewed'
      : (actions.review.assignee ?? 'anyone');
  }
  return draft;
};

export const ruleToDraft = (rule: Rule): RuleDraft =>
  toDraft(rule.conditions, rule.actions);

/** `"12,5"` or `"$12.50"` as the API's `"12.50"`, or `null` when it isn't an amount. */
const toAmount = (text: string): string | null => {
  const cleaned = text.replace(/[$\s]/g, '').replace(',', '.');
  if (!/^\d+(\.\d{0,2})?$/.test(cleaned)) return null;
  return Number(cleaned).toFixed(2);
};

/** The conditions the draft sets — only the switched-on, filled-in ones — or `null` while none is
 * (the rule can't be saved or previewed yet). */
export const draftConditions = (draft: RuleDraft): RuleConditions | null => {
  const statement = draft.statement.filter((text) => text.value.trim() !== '');
  const amount = toAmount(draft.amount);
  const amountMax = toAmount(draft.amountMax);
  const amountComplete =
    amount !== null &&
    (draft.amountOperator !== 'between' ||
      (amountMax !== null && Number(amountMax) >= Number(amount)));

  const conditions: RuleConditions = {
    original_statement:
      draft.statementOn && statement.length > 0 ? statement : null,
    merchant_ids:
      draft.merchantsOn && draft.merchantIds.length > 0
        ? draft.merchantIds
        : null,
    amount:
      draft.amountOn && amountComplete
        ? {
            operator: draft.amountOperator,
            value: amount,
            value_max: draft.amountOperator === 'between' ? amountMax : null,
          }
        : null,
    category_ids:
      draft.categoriesOn && draft.categoryIds.length > 0
        ? draft.categoryIds
        : null,
    owners: draft.ownersOn && draft.owners.length > 0 ? draft.owners : null,
    account_ids:
      draft.accountsOn && draft.accountIds.length > 0 ? draft.accountIds : null,
  };
  return Object.values(conditions).some((value) => value !== null)
    ? conditions
    : null;
};

export const draftActions = (draft: RuleDraft): RuleActions => ({
  merchant_id: draft.renameOn ? draft.merchantId : null,
  category_id: draft.categoryOn ? draft.categoryId : null,
  owner: draft.ownerOn ? draft.owner : null,
  tag_ids: draft.tagsOn && draft.tagIds.length > 0 ? draft.tagIds : null,
  review: draft.reviewOn
    ? draft.review === 'reviewed'
      ? { reviewed: true, assignee: null }
      : { reviewed: false, assignee: draft.review }
    : null,
});

export const hasAction = (actions: RuleActions) =>
  Object.values(actions).some((value) => value !== null);

/** Whether a switched-on condition is still incomplete — saving would silently drop it. */
export const hasIncompleteCondition = (draft: RuleDraft): boolean => {
  const conditions = draftConditions(draft);
  return (
    (draft.statementOn && !conditions?.original_statement) ||
    (draft.merchantsOn && !conditions?.merchant_ids) ||
    (draft.amountOn && !conditions?.amount) ||
    (draft.categoriesOn && !conditions?.category_ids) ||
    (draft.ownersOn && !conditions?.owners) ||
    (draft.accountsOn && !conditions?.account_ids)
  );
};
