export type SortOrder = 'date' | 'inverse_date' | 'amount' | 'inverse_amount';

export interface Transaction {
  id: number;
  date: string;
  merchant: string;
  amount: string;
  category_id: number;
  category_name_en: string;
  category_name_fr: string;
  category_type: 'income' | 'expense' | 'transfer';
  account: string;
  reviewed: boolean;
  created_at: string;
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
