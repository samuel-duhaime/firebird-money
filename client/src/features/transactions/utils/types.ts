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
  account: string;
  reviewed: boolean;
  created_at: string;
  tags: TransactionTag[];
}

export type ImportJobStatus = 'pending' | 'running' | 'succeeded' | 'failed';

export interface ImportJob {
  id: string;
  status: ImportJobStatus;
  file_name: string;
  created_count: number | null;
  failed_count: number | null;
  skipped_count: number | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}
