export interface Merchant {
  id: number;
  /** `null` for a common merchant, shared by every household and immune to rename/delete through
   * this API — everything else belongs to exactly one household. */
  household_id: number | null;
  name: string;
  /** Alternate spellings this merchant also matches on (e.g. "McDonalds" for "McDonald's") —
   * display-only here, not editable through this API yet. */
  aliases: string[];
  logo_url: string | null;
  /** The category most often used with this merchant in this household's own transactions, ties
   * broken by whichever was used most recently. `null` until this household has at least one
   * transaction with this merchant — never settable directly. */
  recommended_category_id: number | null;
  created_at: string;
  /** How many of this household's transactions currently carry this merchant. */
  transaction_count: number;
}

export type MerchantSortOrder = 'transaction_count' | 'alphabetical';
