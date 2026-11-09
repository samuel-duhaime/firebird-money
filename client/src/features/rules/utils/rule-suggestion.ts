import { toast } from 'sonner';
import i18n from '../../../i18n';
import { queryClient } from '../../../lib/query-client';
import { toOwner } from '../../accounts/utils/api';
import { memberDisplayName } from '../../members/utils/format';
import type { Member } from '../../members/utils/types';
import type { TransactionPatch } from '../../transactions/utils/api';
import type { Transaction } from '../../transactions/utils/types';
import { toDraft } from './draft';
import type { RuleDraft } from './draft';
import { suggestRule } from './suggested-rule';

/** The rule a single transaction edit suggests: "if the merchant is what it was, do the same
 * again", or `null` when the edit isn't one a rule can repeat (an amount, a date, a review). */
const suggestedDraft = (
  before: Transaction,
  updated: Transaction,
  patch: TransactionPatch,
): RuleDraft | null => {
  const addedTagIds = (patch.tag_ids ?? []).filter(
    (id) => !before.tags.some((tag) => tag.id === id),
  );
  const merchantChanged =
    patch.merchant_id !== undefined &&
    updated.merchant_id !== before.merchant_id;
  const categoryChanged =
    patch.category_id !== undefined &&
    updated.category_id !== before.category_id;
  const ownerChanged =
    patch.owner !== undefined && updated.owner_user_id !== before.owner_user_id;
  if (
    !merchantChanged &&
    !categoryChanged &&
    !ownerChanged &&
    addedTagIds.length === 0
  ) {
    return null;
  }
  return toDraft(
    { merchant_ids: [before.merchant_id] },
    {
      merchant_id: merchantChanged ? updated.merchant_id : null,
      category_id: categoryChanged ? updated.category_id : null,
      owner: ownerChanged ? toOwner(updated.owner_user_id) : null,
      tag_ids: addedTagIds.length > 0 ? addedTagIds : null,
    },
  );
};

/** What the edit changed it to, e.g. "Restaurants" or "Food, Vacation". */
const newValue = (
  updated: Transaction,
  patch: TransactionPatch,
  draft: RuleDraft,
) => {
  if (draft.renameOn) return updated.merchant_name;
  if (draft.categoryOn) {
    return i18n.language === 'fr'
      ? updated.category_name_fr
      : updated.category_name_en;
  }
  if (draft.ownerOn) {
    if (updated.owner_user_id === null) return i18n.t('settings.rules.shared');
    const member = queryClient
      .getQueryData<Member[]>(['members'])
      ?.find((m) => m.id === updated.owner_user_id);
    return member ? memberDisplayName(member) : '';
  }
  return updated.tags
    .filter(
      (tag) =>
        (patch.tag_ids ?? []).includes(tag.id) && draft.tagIds.includes(tag.id),
    )
    .map((tag) => tag.name)
    .join(', ');
};

/** After a single transaction edit a rule could repeat, offers to create that rule: "Create
 * rule" opens the rules page with it filled in. */
export const ruleSuggestionToast = (
  before: Transaction,
  updated: Transaction,
  patch: TransactionPatch,
  openRules: () => void,
) => {
  const draft = suggestedDraft(before, updated, patch);
  if (!draft) return;
  // Nothing to name, e.g. the added tags aren't in the response (another edit removed them).
  const value = newValue(updated, patch, draft);
  if (value === '') return;
  toast.success(i18n.t('toast.ruleSuggestion', { value }), {
    description: i18n.t('toast.ruleSuggestionDescription'),
    // Long enough to read it and decide.
    duration: 10000,
    action: {
      label: i18n.t('toast.ruleSuggestionAction'),
      onClick: () => {
        suggestRule(draft);
        openRules();
      },
    },
  });
};
