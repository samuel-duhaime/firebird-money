import type { RuleDraft } from './draft';

/** A rule suggested from a transaction edit ("Create rule" on its toast), waiting for the rules
 * page to open it. Held in memory only: a reload just drops the suggestion. */
let suggested: RuleDraft | null = null;

export const suggestRule = (draft: RuleDraft) => {
  suggested = draft;
};

/** The suggested rule, if any. Read it while rendering, then `clearSuggestedRule` once shown —
 * not both in one, since React may render a component twice before keeping it. */
export const suggestedRule = (): RuleDraft | null => suggested;

export const clearSuggestedRule = () => {
  suggested = null;
};
