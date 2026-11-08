import type { RuleDraft } from './draft';

/** A rule suggested from a transaction edit ("Create rule" on its toast), waiting for the rules
 * page to open it. Held in memory only: a reload just drops the suggestion. */
let suggested: RuleDraft | null = null;
const listeners = new Set<() => void>();

export const suggestRule = (draft: RuleDraft) => {
  suggested = draft;
  listeners.forEach((listener) => listener());
};

/** Calls `listener` on each new suggestion, for a rules page that's already open (opening it
 * again wouldn't remount it). Returns the unsubscribe. */
export const onSuggestedRule = (listener: () => void) => {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
};

/** The suggested rule, if any. Read it while rendering, then `clearSuggestedRule` once shown —
 * not both in one, since React may render a component twice before keeping it. */
export const suggestedRule = (): RuleDraft | null => suggested;

export const clearSuggestedRule = () => {
  suggested = null;
};
