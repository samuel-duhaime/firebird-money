export type SortOrder = 'date' | 'inverse_date' | 'amount' | 'inverse_amount';

/** A tag attached to a transaction, as embedded in `Transaction.tags` — just enough to render it. */
export interface TransactionTag {
  id: number;
  name: string;
  color: string;
}

export interface Transaction {
  id: number;
  date: string;
  /** The raw payee/merchant text from the bank or import file — kept forever as the immutable
   * historical record, never user-editable after creation. `merchant_id`/`merchant_name` are what
   * the UI reads/writes. */
  original_statement: string;
  merchant_id: number;
  merchant_name: string;
  merchant_logo_url: string | null;
  amount: string;
  category_id: number;
  category_name_en: string;
  category_name_fr: string;
  category_type: 'income' | 'expense' | 'transfer';
  /** The household account (see Settings → Accounts) it's in. */
  account_id: number;
  account_name: string;
  /** The member it belongs to, or `null` when it's Shared. Starts as its account's owner and
   * follows it, unless it was changed on its own. */
  owner_user_id: number | null;
  reviewed: boolean;
  /** Who it waits on for a review, or `null` for Anyone. Only meaningful while `reviewed` is
   * false. */
  review_assignee_user_id: number | null;
  /** Who marked it reviewed, and when. Both `null` when nobody did, e.g. a transaction added by
   * hand or imported, which starts out reviewed. */
  reviewed_by_user_id: number | null;
  reviewed_at: string | null;
  /** `import` when added from a bank statement, else `manual`. */
  source: 'manual' | 'import';
  created_at: string;
  tags: TransactionTag[];
}
