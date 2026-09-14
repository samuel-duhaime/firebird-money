export interface Category {
  id: number;
  group_id: number;
  name_en: string;
  name_fr: string;
  type: 'income' | 'expense' | 'transfer';
  created_at: string;
}

/** A group of categories, e.g. "Food & Dining" — every category inside it shares its `type`. */
export interface CategoryGroup {
  id: number;
  name_en: string;
  name_fr: string;
  type: 'income' | 'expense' | 'transfer';
  created_at: string;
}
