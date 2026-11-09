import { apiFetch } from '../../../lib/api-client';
import type { Transaction } from '../../transactions/utils/types';
import type { Rule, RuleActions, RuleConditions, RuleOutcome } from './types';

/** Body for `POST /rules` and `PATCH /rules/{id}`. */
export interface RuleInput {
  conditions: RuleConditions;
  actions: RuleActions;
  /** Also apply the rule to the household's existing transactions that match it. */
  apply_to_existing: boolean;
}

export const createRule = (input: RuleInput): Promise<Rule> =>
  apiFetch<Rule>('/rules', {
    method: 'POST',
    body: JSON.stringify(input),
  });

export const updateRule = (id: number, input: RuleInput): Promise<Rule> =>
  apiFetch<Rule>(`/rules/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  });

export const deleteRule = (id: number): Promise<void> =>
  apiFetch<void>(`/rules/${id}`, { method: 'DELETE' });

/** Sets the order the household's rules run in to exactly `ruleIds` and returns them in it. */
export const reorderRules = (ruleIds: number[]): Promise<Rule[]> =>
  apiFetch<Rule[]>('/rules/reorder', {
    method: 'PATCH',
    body: JSON.stringify({ rule_ids: ruleIds }),
  });

export interface RulePreview {
  /** How many of the household's transactions match. */
  match_count: number;
  /** How many of those the rule would change. */
  change_count: number;
  /** The matching transactions, most recent first (at most 500). */
  transactions: { before: Transaction; after: RuleOutcome }[];
}

/** The household's transactions a rule (saved or not) matches, with what it would change. Stores
 * nothing. */
export const previewRule = (
  conditions: RuleConditions,
  actions: RuleActions,
): Promise<RulePreview> =>
  apiFetch<RulePreview>('/rules/preview', {
    method: 'POST',
    body: JSON.stringify({ conditions, actions }),
  });
