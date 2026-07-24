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
  created_at: string;
}
