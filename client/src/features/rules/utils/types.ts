import type { Owner } from '../../accounts/utils/api';
import type { ReviewAssignee } from '../../transactions/utils/api';

export type TextOperator = 'contains' | 'equals';

export type AmountOperator =
  'greater_than' | 'less_than' | 'equal_to' | 'between';

export interface TextCondition {
  operator: TextOperator;
  value: string;
}

export interface AmountCondition {
  operator: AmountOperator;
  value: string;
  /** Inclusive upper bound, only for `between`. */
  value_max: string | null;
}

/** What a transaction must match. Every condition that's set must match; within a list, any one
 * value is enough. `null` means "don't check". */
export interface RuleConditions {
  /** Against the raw statement text, ignoring case. */
  original_statement: TextCondition[] | null;
  merchant_ids: number[] | null;
  amount: AmountCondition | null;
  category_ids: number[] | null;
  owners: Owner[] | null;
  account_ids: number[] | null;
}

/** Mark it reviewed, or send it for review to `assignee` (Anyone when `null`). */
export interface ReviewAction {
  reviewed: boolean;
  assignee: ReviewAssignee | null;
}

/** What a matching transaction gets. `null` leaves that field as is. */
export interface RuleActions {
  /** Renames the merchant: moves the transaction to this existing merchant. */
  merchant_id: number | null;
  category_id: number | null;
  owner: Owner | null;
  /** Added to the transaction's tags; never removes one. */
  tag_ids: number[] | null;
  review: ReviewAction | null;
}

export interface Rule {
  id: number;
  conditions: RuleConditions;
  actions: RuleActions;
  /** Rules run top to bottom (lowest first); a lower rule overwrites what a higher one set. */
  sort_order: number;
  /** When the rule was last applied to existing transactions, and how many it changed then. */
  last_applied_at: string | null;
  last_applied_count: number;
  created_at: string;
}

/** What a rule would make one matching transaction. */
export interface RuleOutcome {
  merchant_id: number | null;
  category_id: number | null;
  owner_user_id: number | null;
  tag_ids: number[];
  reviewed: boolean;
  review_assignee_user_id: number | null;
}
