export interface Category {
  id: number;
  name_en: string;
  name_fr: string;
  type: 'income' | 'expense' | 'transfer';
  created_at: string;
}
