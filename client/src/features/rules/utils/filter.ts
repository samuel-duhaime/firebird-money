import type { Owner } from '../../accounts/utils/api';
import type { RuleLookups } from '../hooks/use-rule-lookups';
import type { Rule } from './types';

/** What the rules page narrows its list to. Empty lists mean "any". */
export interface RuleFilters {
  merchantIds: number[];
  categoryIds: number[];
  accountIds: number[];
  owners: Owner[];
  tagIds: number[];
}

export const NO_RULE_FILTERS: RuleFilters = {
  merchantIds: [],
  categoryIds: [],
  accountIds: [],
  owners: [],
  tagIds: [],
};

export const activeFilterCount = (filters: RuleFilters) =>
  Object.values(filters).filter((values) => values.length > 0).length;

const any = <V>(wanted: V[], used: V[]) =>
  wanted.length === 0 || wanted.some((value) => used.includes(value));

/** Whether `rule` names something in every filter (in a condition or an action) and contains
 * `search` in any of its texts or names. */
export const ruleMatchesFilters = (
  rule: Rule,
  filters: RuleFilters,
  search: string,
  lookups: RuleLookups,
) => {
  const { conditions, actions } = rule;
  const merchantIds = [
    ...(conditions.merchant_ids ?? []),
    ...(actions.merchant_id !== null ? [actions.merchant_id] : []),
  ];
  const categoryIds = [
    ...(conditions.category_ids ?? []),
    ...(actions.category_id !== null ? [actions.category_id] : []),
  ];
  const accountIds = conditions.account_ids ?? [];
  const owners = [
    ...(conditions.owners ?? []),
    ...(actions.owner !== null ? [actions.owner] : []),
  ];
  const tagIds = actions.tag_ids ?? [];

  if (
    !any(filters.merchantIds, merchantIds) ||
    !any(filters.categoryIds, categoryIds) ||
    !any(filters.accountIds, accountIds) ||
    !any(filters.owners, owners) ||
    !any(filters.tagIds, tagIds)
  ) {
    return false;
  }

  const term = search.trim().toLowerCase();
  if (term === '') return true;
  const texts = [
    ...(conditions.original_statement ?? []).map((text) => text.value),
    ...merchantIds.map(lookups.merchantName),
    ...categoryIds.map(lookups.categoryName),
    ...accountIds.map(lookups.accountName),
    ...owners.map(lookups.ownerName),
    ...tagIds.map((id) => lookups.tag(id)?.name ?? ''),
  ];
  return texts.some((text) => text.toLowerCase().includes(term));
};
